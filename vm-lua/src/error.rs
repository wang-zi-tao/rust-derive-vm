use failure::Error;

#[derive(Fail, Debug)]
pub enum LuaVMError {
    #[fail(display = "syntax error:{}", _0)]
    SyntaxError(#[cause] Error),
    #[fail(display = "other error:{}", _0)]
    Other(#[cause] Error),
}
