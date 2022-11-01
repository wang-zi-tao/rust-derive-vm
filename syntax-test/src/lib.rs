#[cfg(test)]
mod test {
    use failure::Fallible;
    use syntax_derive::{lalr1_analyser, recursive_predictive_analysis};

    #[derive(Debug)]
    enum LexicalDemo {
        Float(f64),
        Add,
        Sub,
    }
    use LexicalDemo::*;

    #[test]
    fn test_ll1() -> Fallible<()> {
        recursive_predictive_analysis! {
          parser:LexicalDemo->f64{
            expr=>f64 ->{
                [Sub,Float(i1)] => Ok(-i1);
              | [Float(i1),expr_extend(i2)]=>Ok(i1+i2);
            },
            expr_extend=>f64->{
                [Add,Float(i2),expr_extend(i1)]=>Ok(i2+i1);
              | [Sub,Float(i2),expr_extend(i1)]=>Ok(i1-i2);
              | []=>Ok(0.0);
            },
            unused->{
              [Add]|[Sub]
            }
          }
        };
        let r = parser(&[Float(2.0), Sub, Float(1.0)])?;
        assert_eq!(r, 1.0);
        Ok(())
    }
    #[test]
    fn test_lalr1() -> Fallible<()> {
        lalr1_analyser! {
          parser:LexicalDemo->String{
            syn=>f64->{[expr(v)]=>Ok(v);},
            expr=>f64 ->{
              [Float(v1)]=>Ok(v1);
              | [expr(v2),Add,Float(v1)]=>Ok(v1+v2);
            },
          }
        };
        let r = parser(vec![Float(2.0), Add, Float(1.0)])?;
        assert_eq!(r, 3.0);
        Ok(())
    }
}
