pub fn camel_case_ident_to_snake_case_ident(ident: &str) -> String {
    let mut result = String::new();
    for (i, c) in ident.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}
pub fn camel_case_ident_to_upper_snake_case_ident(ident: &str) -> String {
    let mut result = String::new();
    for (i, c) in ident.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            result.push('_');
        }
        result.push(c.to_ascii_uppercase());
    }
    result
}
pub fn to_camel_case(ident: &str) -> String {
    let mut result = String::new();
    let mut word_start = true;
    for (_i, c) in ident.chars().enumerate() {
        if word_start {
            word_start = false;
            result.push(c.to_ascii_uppercase());
        } else if c == '_' {
            word_start = true;
        } else {
            result.push(c);
        }
    }
    result
}
