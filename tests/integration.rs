use ers_templates::Renderable;
use std::fmt::Display;

#[derive(Renderable)]
#[Template(path="tests/test.html", open="<?%", close="%>")]
struct Test<'a>{
    title: &'a str,
    class: &'a str
}

#[test]
fn test_parsing(){
    let test = Test{
        title: "I'm a title",
        class: "class"
    };
    println!("{}", test)
}
