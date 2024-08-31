use std::{ops::Deref, sync::Arc};

use arc_swap::ArcSwap;
use axum::http::Method;
use matchit::{Match, Router};

use crate::{config::ProjectRoutes, error::AppError};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MethodRoute {
    get: Option<String>, // handler name in js code
    head: Option<String>,
    delete: Option<String>,
    options: Option<String>,
    patch: Option<String>,
    post: Option<String>,
    put: Option<String>,
    trace: Option<String>,
    connect: Option<String>,
}

#[derive(Clone)]
pub struct SwappableAppRouter {
    pub inner: Arc<ArcSwap<AppRouterInner>>,
}

pub struct AppRouterInner {
    pub code: String,
    pub router: Router<MethodRoute>,
}

impl SwappableAppRouter {
    pub fn try_new(code: impl Into<String>, routes: ProjectRoutes) -> anyhow::Result<Self> {
        let router = Self::get_router(routes)?;
        let inner = AppRouterInner::new(code, router);
        Ok(Self {
            inner: Arc::new(ArcSwap::from_pointee(inner)),
        })
    }

    pub fn swap(&self, code: impl Into<String>, routes: ProjectRoutes) -> anyhow::Result<()> {
        let router = Self::get_router(routes)?;
        let inner = AppRouterInner::new(code, router);
        self.inner.store(Arc::new(inner));
        Ok(())
    }

    pub fn load(&self) -> AppRouter {
        AppRouter(self.inner.load_full())
    }

    fn get_router(routes: ProjectRoutes) -> anyhow::Result<Router<MethodRoute>> {
        let mut router = Router::new();
        for (path, methods) in routes {
            let mut method_route = MethodRoute::default();
            for method in methods {
                match method.method {
                    Method::GET => method_route.get = Some(method.handler),
                    Method::HEAD => method_route.head = Some(method.handler),
                    Method::DELETE => method_route.delete = Some(method.handler),
                    Method::OPTIONS => method_route.options = Some(method.handler),
                    Method::PATCH => method_route.patch = Some(method.handler),
                    Method::POST => method_route.post = Some(method.handler),
                    Method::PUT => method_route.put = Some(method.handler),
                    Method::TRACE => method_route.trace = Some(method.handler),
                    Method::CONNECT => method_route.connect = Some(method.handler),
                    v => unreachable!("unsupported method {v}"),
                }
            }
            router.insert(path, method_route)?;
        }
        Ok(router)
    }
}

#[derive(Clone)]
pub struct AppRouter(Arc<AppRouterInner>);

impl AppRouter {
    pub fn match_it<'m, 'p>(
        &'m self,
        method: Method,
        path: &'p str,
    ) -> Result<Match<&str>, AppError>
    where
        'p: 'm,
    {
        let Ok(ret) = self.router.at(path) else {
            return Err(AppError::RoutePathNotFound(path.to_string()));
        };
        let s = match method {
            Method::GET => ret.value.get.as_deref(),
            Method::HEAD => ret.value.head.as_deref(),
            Method::DELETE => ret.value.delete.as_deref(),
            Method::OPTIONS => ret.value.options.as_deref(),
            Method::PATCH => ret.value.patch.as_deref(),
            Method::POST => ret.value.post.as_deref(),
            Method::PUT => ret.value.put.as_deref(),
            Method::TRACE => ret.value.trace.as_deref(),
            Method::CONNECT => ret.value.connect.as_deref(),
            _ => unreachable!(),
        }
        .ok_or(AppError::RouteMethodNotAllowed(method))?;
        Ok(Match {
            value: s,
            params: ret.params,
        })
    }
}

impl Deref for AppRouter {
    type Target = AppRouterInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppRouterInner {
    pub fn new(code: impl Into<String>, router: Router<MethodRoute>) -> Self {
        Self {
            code: code.into(),
            router,
        }
    }
}

#[cfg(test)]
mod tests {
    use matchit::Router;

    use crate::config::ProjectConfig;

    use super::*;

    #[test]
    fn test_base() -> anyhow::Result<()> {
        let mut router = Router::new();
        router.insert("/aaa", "aaa")?;
        router.insert("/b{id}", "bbb")?;
        let res = router.at("/baa")?;
        assert_eq!(res.params.get("id"), Some("aa"));
        let res = router.at("/aaa")?;
        assert_eq!(*res.value, "aaa");
        Ok(())
    }

    #[test]
    fn test_method_route() -> anyhow::Result<()> {
        let mut router = Router::new();
        let method_route = MethodRoute {
            get: Some("get".to_string()),
            post: Some("post".to_string()),
            ..Default::default()
        };
        router.insert("/aaa", method_route.clone())?;
        router.insert("/bbb/{*id}", method_route.clone())?;
        let res = router.at("/aaa")?;
        assert_eq!(res.value, &method_route);

        Ok(())
    }

    #[test]
    fn test_app_route() -> anyhow::Result<()> {
        let mut router = Router::new();
        let method_route = MethodRoute {
            get: Some("get".to_string()),
            ..Default::default()
        };
        router.insert("/bbb/{*id}", method_route)?;

        let inner = AppRouterInner::new("", router);
        let app_router = AppRouter(Arc::new(inner));
        let res = app_router.match_it(Method::GET, "/bbb/123")?;
        assert_eq!(res.value, "get");
        assert_eq!((res.params.get("id")), Some("123"));
        Ok(())
    }

    #[test]
    fn app_router_swap_should_work() -> anyhow::Result<()> {
        let config: ProjectConfig = serde_yaml::from_str(
            r#"
        name: dino-test
        routes:
          /api/hello/{id}:
            - method: GET
              handler: hello1
        "#,
        )?;

        let router = SwappableAppRouter::try_new("", config.routes)?;
        let app_router = router.load();
        let m = app_router.match_it(Method::GET, "/api/hello/123")?;
        assert_eq!(m.value, "hello1");

        let newconfig: ProjectConfig = serde_yaml::from_str(
            r#"
        name: dino-test
        routes:
          /api/hello/{id}:
            - method: GET
              handler: hello1
          /api/goodbye/{id}:
            - method: POST
              handler: handler2
        "#,
        )?;
        router.swap("", newconfig.routes)?;
        let app_router = router.load();
        let m = app_router.match_it(Method::GET, "/api/hello/123")?;
        assert_eq!(m.value, "hello1");
        let m = app_router.match_it(Method::POST, "/api/goodbye/123")?;
        assert_eq!(m.value, "handler2");
        Ok(())
    }
}
