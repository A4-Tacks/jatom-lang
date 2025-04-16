use crate::Arc;
use ordered_float::OrderedFloat;

macro_rules! impl_enum_froms {
    (impl From for $ty:ty { $(
        $variant:ident => $target:ty
    );* $(;)? }) => { $(
        impl From<$target> for $ty {
            fn from(value: $target) -> Self {
                Self::$variant(value.into())
            }
        }
    )* };
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    InvalidUnicode(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Expr {
    pub value: Arc<ExprValue>,
    pub location: (usize, usize),
}
impl Expr {
    pub fn new(value: Arc<ExprValue>, location: (usize, usize)) -> Self {
        Self { value, location }
    }
}
impl std::ops::Deref for Expr {
    type Target = ExprValue;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExprValue {
    Pipe(Vec<Expr>),
    Op1(SingleOp, Expr),
    Op2(BinaryOp, Expr, Expr),
    And(Expr, Expr),
    Or(Expr, Expr),
    If(If),
    Call(Expr, Vec<Expr>),
    Literal(Literal),
    Ident(Arc<str>),
}
impl_enum_froms!(impl From for ExprValue {
    Pipe => Vec<Expr>;
    Literal => Literal;
    If => If;
    Ident => &'_ str;
});

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SingleOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    IDiv,
    Rem,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct If {
    cond: Expr,
    yes: Expr,
    no: Option<Expr>,
}
impl If {
    pub fn new(cond: Expr, yes: Expr, no: Option<Expr>) -> Self {
        Self { cond, yes, no }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Literal {
    String(Arc<str>),
    Number(OrderedFloat<f64>),
}
impl Literal {
    /// # Panics
    /// - escape body contains multi bytes char
    /// - invalid escape hex code
    pub fn escape(s: &str) -> Result<Self, Error> {
        let Some((acc, mut s)) = s.split_once('\\') else {
            return Ok(s.into());
        };
        let mut acc = acc.to_owned();
        acc.reserve(s.len());

        let p = |s| u32::from_str_radix(s, 16).unwrap();
        loop {
            let (escaped, skips) = match &s[..1] {
                "\\" => ('\\', 1),
                "\"" => ('"', 1),
                "n" => ('\n', 1),
                "r" => ('\r', 1),
                "b" => ('\x08', 1),
                "t" => ('\t', 1),
                "e" => ('\x1b', 1),
                "x" => (
                    char::from_u32(p(&s[1..3])).unwrap(),
                    3,
                ),
                "u" => (
                    char::from_u32(p(&s[1..5])).unwrap(),
                    5,
                ),
                "U" => {
                    let code = p(&s[1..9]);
                    let Some(ch) = char::from_u32(code) else {
                        return Err(Error::InvalidUnicode(code));
                    };
                    (ch, 9)
                },
                _ => unreachable!("{s}"),
            };
            acc.push(escaped);
            s = &s[skips..];
            if let Some((processed, rem)) = s.split_once('\\') {
                acc.push_str(processed);
                s = rem;
            } else { break }
        }

        acc.push_str(s);
        Ok(Self::String(acc.into()))
    }
}
impl From<Arc<&'_ str>> for Literal {
    fn from(value: Arc<&'_ str>) -> Self {
        Self::String((*value).into())
    }
}
impl_enum_froms!(impl From for Literal {
    Number => f64;
    Number => OrderedFloat<f64>;
    String => &'_ str;
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::*;

    #[test]
    fn test_escape() {
        let srcs = [
            (r#""#, ""),
            (r#"a"#, "a"),
            (r#"ab"#, "ab"),
            (r#"abc"#, "abc"),
            (r#"abc\n"#, "abc\n"),
            (r#"abc\nd"#, "abc\nd"),
            (r#"abc\ndef"#, "abc\ndef"),
            (r#"abc\ndef\""#, "abc\ndef\""),
            (r#"abc\n\ndef\""#, "abc\n\ndef\""),
            (r#"abc\n\\\ndef\""#, "abc\n\\\ndef\""),
            (r#"abc\x1b\n\\\ndef\""#, "abc\x1b\n\\\ndef\""),
            (r#"abc\e\n\\\ndef\""#, "abc\x1b\n\\\ndef\""),
            (r#"abc\u001b\n\\\ndef\""#, "abc\x1b\n\\\ndef\""),
            (r#"abc\U0000001b\n\\\ndef\""#, "abc\x1b\n\\\ndef\""),
            (r#"\n"#, "\n"),
            (r#"\nq"#, "\nq"),
            (r#"\nab"#, "\nab"),
        ];

        for (src, expected) in srcs {
            assert_eq!(Literal::escape(src),
                    Ok(Literal::String(expected.into())));
        }
    }

    #[test]
    fn it_works() {
        let parser = AtomParser::new();
        let srcs = [
            "x",
            "_x",
            "x_y",
            "--1.2e3",
            "1",
            "(1)",
            "(-1)",
            "(1 2)",
            "(1 -2)",
            "(1 2 3)",
            "if 1 2",
            "if 1 2 else 3",
            "if -1 -2 else -3",
            "if --1 --2 else --3",
            "if 1 2 else if 3 4",
            "if 1 2 else if 3 4 else 5",
            "if --1 (--2 3) else (--3 4)",
            "if --1 if 2 3 else 4 else 5",
            "if --1 if 2 if 3 4",
            "if --1 if 2 if 3 4 else 5",
            "if --1 if 2 if 3 4 else 5 else 6",
            "if --1 if 2 if 3 4 else 5 else 6 else 7",
            "if --1 if 2 if 3 4 else 5 else 6 else 7 # foo",
            "if --1 if 2 if 3 4 else 5 else 6 else 7 # foo测试",
            "if --1 if 2 if 3 4 else 5 else 6 else # foo测试\n 7",
            "{1==2}",
            "{1<2==2<3}",
            "{1<2==2<3+1}",
            "{1<2==2<3+1;2}",
            "{1<2==2<3+1;2-3}",
            "{1<2==2<3+1;2-3*2}",
            "{1<2==2<3+1;2-3*2;5*x}",
            "if  {a<b} 1 else 2",
            "if  {a<b} -1 else 2",
            "{if {a<b} -1 else 2}",
            "{if {a<b} -1 else -2}",
            "{if {a<b} -1 else -2;4}",
            "{if {a<b} f(2) else -2;4}",
            "{if {a<b} f(2).3 else -2;4}",
            "{if {a<b} f(2).-3 else -2;4}",
            "{if {a<b} f(2).--3 else -2;4}",
            "{if {a<b} f(2).f(3) else -2;4}",
            "{if {a<b} f(2).{f(3)} else -2;4}",
            "if  a<b 1 else 2",
            "if  a<b -1 else 2",
            "{if a<b -1 else 2}",
            "{if a<b -1 else -2}",
            "{if a<b -1 else -2;4}",
            "{if a<b f(2) else -2;4}",
            "{if a<b f(2).3 else -2;4}",
            "{if a<b f(2).-3 else -2;4}",
            "{if a<b f(2).--3 else -2;4}",
            "{if a<b f(2).f(3) else -2;4}",
            "{if a<b f(2).{f(3)} else -2;4}",
            "if  a<b 1 else 2",
            "if  a<b && c || d -1 else 2",
            "{if a<b && c || d -1 else 2}",
            "{if a<b && c || d -1 else -2}",
            "{if a<b && c || d -1 else -2;4}",
            "{if a<b && c || d f(2) else -2;4}",
            "{if a<b && c || d f(2).3 else -2;4}",
            "{if a<b && c || d f(2).-3 else -2;4}",
            "{if a<b && c || d f(2).--3 else -2;4}",
            "{if a<b && c || d f(2).f(3) else -2;4}",
            "{if a<b && c || d f(2).{f(3)} else -2;4}",
            "{if a<-b && !c || d f(2).{f(3)} else -2;4}",
            "{if a<b&&{c||d} m}",
            "{'a'.'b'}",
            "'a'.'b'",
            "fmt.'b'",
            "('a'fmt.'b')",
        ];
        for src in srcs {
            parser.parse(src).expect(src);
        }
    }
}
