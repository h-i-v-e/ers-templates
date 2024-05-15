use std::borrow::Cow;
use std::panic;
use std::str::SplitWhitespace;
use regex::Captures;

use regex::Regex;

enum ExpressionType{
    Comment, HtmlEscaped, Raw, Open, Close, Escaped
}

struct Expression<'a>{
    expression_type: ExpressionType,
    preffix: &'a str,
    content: &'a str,
    postfix: &'a str
}

impl<'a> Expression<'a>{
    fn close(expression_type: ExpressionType, preffix: &'a str, start: &'a str, end: &'static str) -> Option<Self>{
        match start.find(end){
            Some(mut pos) => {
                let mut postfix = &start[pos + end.len() ..];
                if &start[pos - 1 .. pos] == "~"{
                    postfix = postfix.trim_start();
                    pos -= 1;
                } 
                Some(Self { expression_type, preffix, content: &start[.. pos], postfix })
            },
            None => None
        }
    }

    fn check_comment(preffix: &'a str, start: &'a str) -> Option<Self>{
        if let Some(pos) = start.find("--"){
            if pos == 0{
                return Self::close(ExpressionType::Comment, preffix, &start[2 ..], "--}}");
            }
        }
        Self::close(ExpressionType::Comment, preffix, start, "}}")
    }

    fn from(src: &'a str) -> Option<Self>{
        match src.find("{{"){
            Some(start) => {
                let mut second = start + 3;
                if second >= src.len(){
                    return None;
                }
                if start > 0 && &src[start - 1 .. start] == "\\"{
                    return Self::close(ExpressionType::Escaped, &src[.. start - 1], &src[second - 1 ..], "}}");
                }
                let mut prefix = &src[.. start];
                let mut marker = &src[start + 2 .. second];
                if marker == "~"{
                    prefix = prefix.trim_end();
                    second += 1;
                    if second >= src.len(){
                        return None;
                    }
                    marker = &src[start + 3 .. second];
                }
                match marker{
                    "{" => Self::close(ExpressionType::Raw, prefix, &src[second ..], "}}}"),
                    "!" => Self::check_comment(prefix, &src[second ..]),
                    "#" => Self::close(ExpressionType::Open, prefix, &src[second ..], "}}"),
                    "/" => Self::close(ExpressionType::Close, prefix, &src[second ..], "}}"),
                    _ => Self::close(ExpressionType::HtmlEscaped, prefix, &src[second - 1 ..], "}}")
                }
            },
            None => None
        }
    }

    fn next(&self) -> Option<Self>{
        Self::from(self.postfix)
    }
}

#[derive(Debug)]
enum OpenType{
    If, Else, Unless, Each, With
}

#[derive(Debug)]
struct Scope<'a>{
    opened: OpenType,
    this: Option<&'a str>,
    local: Option<&'a str>
}

struct Compile<'a>{
    rust: String,
    open_stack: Vec<Scope<'a>>
}

impl<'a> Compile<'a>{
    fn new() -> Self{
        Self{
            rust: String::new(),
            open_stack: Vec::new()
        }
    }

    /*fn debug_stack(&self){
        for scope in self.open_stack.iter(){
            println!("{:?}", scope);
        }
    }*/

    fn push_scope_with_local(&mut self, opened: OpenType, local: Option<&'a str>){
        self.open_stack.push(Scope{
            opened,
            this: match self.open_stack.last(){
                Some(scope) => scope.this,
                None => None
            },
            local
        });
    }

    fn push_scope(&mut self, opened: OpenType){
        self.push_scope_with_local(opened, None)
    }

    fn find_scope(&self, mut var: &'a str) -> Option<(&'a str, &Scope<'a>)>{
        for scope in self.open_stack.iter().rev(){
            if var.starts_with("../"){
                var = &var[3 ..];
                continue;
            }
            return Some((var, scope));
        }
        None
    }

    fn write_var(&mut self, var: &str){
        match self.find_scope(var){
            Some((var, scope)) => {
                let prefix = match var.find('.'){
                    Some(pos) => &var[.. pos],
                    None => var
                };
                if let Some(local) = scope.local{
                    if local == prefix{
                        self.write(var);
                        return;
                    }
                }
                if let Some(this) = scope.this  {
                    self.rust.push_str(this);
                    self.rust.push('.');   
                }
                self.write(var);
            },
            None => self.write(var)
        }
    }

    fn resolve(&mut self, src: &str, prefix: &str, postfix: &str){
        let mut tokens = src.split_whitespace();
        let var = match tokens.next(){
            Some(token) => token,
            None => return 
        };
        if var == "else"{
            if let Some(scope) = self.open_stack.last() {
                match scope.opened{
                    OpenType::If | OpenType::Unless => {
                        self.open_stack.push(Scope{
                            opened: OpenType::Else,
                            this: scope.this,
                            local: None
                        });
                        self.rust.push_str("}else{");
                        return;
                    },
                    _ => ()
                }
            }
        }
        self.write(prefix);
        self.write_var(var);
        let mut glue = '(';
        while let Some(token) = tokens.next(){
            self.push(glue);
            self.write_var(token);
            glue = ',';
        }
        if glue != '('{
            self.push(')');
        }
        self.write(postfix);
    }

    fn write(&mut self, content: &str){
        self.rust.push_str(content);
    }

    fn push(&mut self, c: char){
        self.rust.push(c);
    }

    fn resolve_if(&mut self, mut tokens: SplitWhitespace<'a>){
        self.write("if ");
        let local = match tokens.next() {
            Some("some") => match tokens.next(){
                Some(var) => {
                    match tokens.next(){
                        Some("as") => {
                            let local = tokens.next().unwrap();
                            self.write("let Some(");
                            self.write(local);
                            self.write(") = ");
                            self.write_var(var);
                            Some(local)
                        },
                        Some(_) => panic!("Invalid block"),
                        None => {
                            self.write_var(var);
                            self.write(".is_some()");
                            None
                        }
                    }
                },
                None => {
                    self.write_var("some");
                    None
                }
            },
            Some(other) => {
                self.write_var(other);
                None
            },
            None => {
                self.write("true");
                None
            }
        };
        self.push_scope_with_local(OpenType::If, local);
        self.push('{');
    }

    fn resolve_unless(&mut self, mut tokens: SplitWhitespace){
        self.write("if ");
        match tokens.next() {
            Some("some") => {
                match tokens.next(){
                    Some(var) => {
                        self.write_var(var);
                        self.write(".is_none()");
                    },
                    None => {
                        self.push('!');
                        self.write_var("some");
                    }
                }
            }
            Some(other) => {
                self.write("!");
                self.write_var(other);
            }
            None => self.write("false")
        }
        self.push('{');
        self.push_scope(OpenType::Unless);
    }

    fn resolve_each(&mut self, mut tokens: SplitWhitespace<'a>){
        let next = tokens.next().unwrap();
        let local = match tokens.next(){
            Some("as") => {
                tokens.next().unwrap()
            },
            Some(_) => panic!("Invalid block"),
            None => "this"
        };
        self.write("for ");
        self.write(local);
        self.write(" in ");
        self.write_var(next);
        self.push('{');
        self.push_scope_with_local(OpenType::Each, Some(local));
    }

    fn resolve_with(&mut self, mut tokens: SplitWhitespace<'a>){
        self.open_stack.push(Scope{
            opened: OpenType::With,
            this: Some(tokens.next().unwrap()),
            local: None
        });
    }

    fn close(&mut self, content: &str){
        //self.debug_stack();
        let scope = match self.open_stack.pop() {
            Some(scope) => scope,
            None => panic!("Mismatched block near {}", content)
        };
        let with = content == "with";
        if !match scope.opened{
            OpenType::If => content == "if",
            OpenType::Else => match self.open_stack.pop(){
                Some(scope) => match scope.opened{
                    OpenType::If | OpenType::Unless => true,
                    _ => false
                },
                None => false
            },
            OpenType::Unless => content == "unless",
            OpenType::Each => content == "each",
            OpenType::With => with
        }{
            panic!("Mismatched block near {}", content);
        }
        if !with{
            self.push('}');
        }
    }

    fn open(&mut self, content: &'a str){
        let mut tokens = content.split_whitespace();
        match tokens.next(){
            Some("if") => self.resolve_if(tokens),
            Some("unless") => self.resolve_unless(tokens),
            Some("each") => self.resolve_each(tokens),
            Some("with") => self.resolve_with(tokens),
            _ => panic!("Invalid block")
        }
    }
} 

pub struct Compiler{
    clean: Regex,
    strip_ws: Regex
}

impl Compiler {
    pub fn new() -> Self{
        Self{
            clean: Regex::new("[\\\\\"]").unwrap(),
            strip_ws: Regex::new(r">\s+<").unwrap()
        }
    }

    fn escape<'a>(&self, content: &'a str) -> Cow<'a, str> {
        self.clean.replace_all(
            &content, |captures: &Captures| format!("\\{}", &captures[0])
        )
    }

    fn write_str(&self, out: &mut Compile, content: &str) {
        if content.is_empty(){
            return;
        }
        let stripped = self.strip_ws.replace_all(&content, "><");
        out.write("f.write_str(\"");
        out.write(self.escape(&stripped).as_ref());
        out.write("\")?;");
    }

    pub fn compile(&self, src: &str) -> String{
        let mut compile = Compile::new();
        let mut suffix = src;
        let mut expression = Expression::from(src);
        while let Some(expr) = expression{
            suffix = expr.postfix;
            self.write_str(&mut compile, expr.preffix);
            match expr.expression_type{
                ExpressionType::Raw => compile.resolve(expr.content, "Display::fmt(&(", "),f)?;"),
                ExpressionType::HtmlEscaped => compile.resolve(expr.content, "rusty_hbs_helpers::escape(&(", "), f)?;"),
                ExpressionType::Open => compile.open(expr.content),
                ExpressionType::Close => compile.close(expr.content.trim()),
                ExpressionType::Escaped => self.write_str(&mut compile, expr.content),
                _ => ()
            }
            expression = expr.next();
        }
        self.write_str(&mut compile, suffix);
        compile.rust
    }
}

#[cfg(test)]
mod tests {
    use crate::hbs_parser::Compiler;

    #[test]
    fn it_works() {
        let compiler = Compiler::new();
        let src = "Hello {{{name}}}!";
        let rust = compiler.compile(src);
        assert_eq!(rust, "f.write_str(\"Hello \")?;Display::fmt(&(name),f)?;f.write_str(\"!\")?;");
    }

    #[test]
    fn test_if(){
        let rust = Compiler::new().compile("{{#if some}}Hello{{/if}}");
        assert_eq!(rust, "if some{f.write_str(\"Hello\")?;}");
    }

    #[test]
    fn test_else(){
        let rust = Compiler::new().compile("{{#if some}}Hello{{else}}World{{/if}}");
        assert_eq!(rust, "if some{f.write_str(\"Hello\")?;}else{f.write_str(\"World\")?;}");
    }

    #[test]
    fn test_unless(){
        let rust = Compiler::new().compile("{{#unless some}}Hello{{/unless}}");
        assert_eq!(rust, "if !some{f.write_str(\"Hello\")?;}");
    }

    #[test]
    fn test_each(){
        let rust = Compiler::new().compile("{{#each some as item}}Hello {{item}}{{/each}}");
        assert_eq!(rust, "for item in some{f.write_str(\"Hello \")?;rusty_hbs_helpers::escape(&(item), f)?;}");
    }

    #[test]
    fn test_with(){
        let rust = Compiler::new().compile("{{#with some}}Hello {{name}}{{/with}}");
        assert_eq!(rust, "f.write_str(\"Hello \")?;rusty_hbs_helpers::escape(&(some.name), f)?;");
    }

    #[test]
    fn test_nesting(){
        let rust = Compiler::new().compile("{{#if some some as some}}{{#each some as item}}Hello {{item}}{{/each}}{{/if}}");
        assert_eq!(rust, "if let Some(some) = some{for item in some{f.write_str(\"Hello \")?;rusty_hbs_helpers::escape(&(item), f)?;}}");
    }

    #[test]
    fn test_comment(){
        let rust = Compiler::new().compile("Note: {{! This is a comment }} and {{!-- {{so is this}} --}}\\{{{{}}");
        assert_eq!(rust, "f.write_str(\"Note: \")?;f.write_str(\" and \")?;f.write_str(\"{{\")?;");
    }

    #[test]
    fn test_scoping(){
        let rust = Compiler::new().compile("{{#with some}}{{#with other}}Hello {{name}} {{../company}} {{/with}}{{/with}}");
        assert_eq!(rust, "f.write_str(\"Hello \")?;rusty_hbs_helpers::escape(&(other.name), f)?;f.write_str(\" \")?;rusty_hbs_helpers::escape(&(some.company), f)?;f.write_str(\" \")?;");
    }

    #[test]
    fn test_trimming(){
        let rust = Compiler::new().compile("  {{~#if some ~}}   Hello{{~/if~}}");
        assert_eq!(rust, "if some{f.write_str(\"Hello\")?;}");
    }
}