use typed_builder::TypedBuilder;

#[allow(unused)]
#[derive(Debug, TypedBuilder)]
struct Foo {
    #[builder(setter(transform=|x: i32| x + 1))]
    x: i32,
    #[builder(setter(prefix = "with_"))]
    y: u32,
}
fn main() -> anyhow::Result<()> {
    let res = Foo::builder().x(1).with_y(77).build();
    println!("{:?}", res);
    Ok(())
}
