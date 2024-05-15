pub mod hbs_parser;

pub trait HtmlEscaped{
    fn escape(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl HtmlEscaped for str{
    fn escape(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.chars(){
            match c{
                '&' => write!(f, "&amp;")?,
                '<' => write!(f, "&lt;")?,
                '>' => write!(f, "&gt;")?,
                c => write!(f, "{}", c)?
            }
        }
        Ok(())
    }
}

impl<T: ToString> HtmlEscaped for T{
    fn escape(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_string().as_str().escape(f)
    }
}

pub fn escape<T: HtmlEscaped>(item: &T, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    item.escape(f)
}

#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use super::*;

    struct Test<T: HtmlEscaped>{
        val: T
    }

    impl<T: HtmlEscaped> Test<T>{
        fn new(val: T) -> Self{
            Self{val}
        }
    }

    impl<T: HtmlEscaped> Display for Test<T>{
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.val.escape(f)
        }
    }

    #[test]
    fn test_escape_str(){
        assert_eq!(Test::new("& Im < 100 years old but > 30").to_string(), "&amp; Im &lt; 100 years old but &gt; 30");
    }

    #[test]
    fn test_escape_int(){
        assert_eq!(Test::new(100).to_string(), "100");
    }
}