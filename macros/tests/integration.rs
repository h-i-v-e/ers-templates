use rusty_hbs::Renderable;
use std::fmt::Display;

#[derive(Renderable)]
#[Template(path="macros/tests/test.hbs")]
struct Test<'a>{
    title: &'a str,
    class: &'a str,
    items: &'a [&'a str]
}

#[test]
fn test_parsing(){
    let test = Test{
        title: "I'm a title",
        class: "class",
        items: &["one < two", "& two < three", "which is > two"]
    };
    println!("{}", test)
}
