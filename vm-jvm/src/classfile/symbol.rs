use util::PooledStr;
#[derive(Clone, Debug)]
pub struct TypeSymbol {
    pub name: PooledStr,
}
impl TypeSymbol {
    pub fn from_class_name(name: PooledStr) -> Option<Self> {
        if check_is_class_name(&*name) {
            Some(Self { name })
        } else {
            None
        }
    }

    pub fn from_type_name(name: PooledStr) -> Option<Self> {
        if check_is_type_name(&*name) {
            Some(Self { name })
        } else {
            None
        }
    }

    pub fn new_unchecked(name: PooledStr) -> Self {
        Self { name }
    }
}
#[derive(Clone, Debug)]
pub struct FieldSymbol {
    pub name: PooledStr,
    pub descriptor: TypeSymbol,
}
impl FieldSymbol {
    pub fn new(name: PooledStr, descriptor: PooledStr) -> Option<Self> {
        if check_is_field_name(&*name) {
            Some(Self {
                name,
                descriptor: TypeSymbol::from_type_name(descriptor)?,
            })
        } else {
            None
        }
    }
}
#[derive(Clone, Debug)]
pub struct MethodTypeSymbol {
    pub return_type: TypeSymbol,
    pub parameters: Vec<TypeSymbol>,
}
impl MethodTypeSymbol {
    pub fn new(descriptor: &PooledStr) -> Option<Self> {
        let mut iter = (&**descriptor).as_bytes().iter().peekable();
        if iter.next() != Some(&b'(') {
            None?
        };
        let mut parameters = Vec::new();
        while iter.peek() != Some(&&b')') {
            parameters.push(TypeSymbol::new_unchecked(
                parser_type_name(&mut iter)?.into(),
            ));
        }
        if iter.next() != Some(&b')') {
            None?
        };
        let return_type = if iter.peek() == Some(&&b'V') && iter.len() == 1 {
            TypeSymbol::new_unchecked("V".into())
        } else {
            let return_type = TypeSymbol::new_unchecked(parser_type_name(&mut iter)?.into());
            if iter.next().is_some() {
                None?
            };
            return_type
        };
        Some(Self {
            return_type,
            parameters,
        })
    }
}
#[derive(Clone, Debug)]
pub struct MethodSymbol {
    pub name: PooledStr,
    pub descriptor: MethodTypeSymbol,
}
impl MethodSymbol {
    pub fn new(name: PooledStr, descriptor: &PooledStr) -> Option<Self> {
        match &*name {
            "<init>" | "<clinit>" => {
                let method_type = MethodTypeSymbol::new(descriptor)?;
                if &*(method_type.return_type.name) == "V" {
                    Some(Self {
                        name,
                        descriptor: method_type,
                    })
                } else {
                    None
                }
            }
            _ => {
                let method_type = MethodTypeSymbol::new(descriptor)?;
                Some(Self {
                    name,
                    descriptor: method_type,
                })
            }
        }
    }

    pub fn new_not_initialization(name: PooledStr, descriptor: &PooledStr) -> Option<Self> {
        match &*name {
            "<init>" | "<clinit>" => None,
            _ => {
                let method_type = MethodTypeSymbol::new(descriptor)?;
                Some(Self {
                    name,
                    descriptor: method_type,
                })
            }
        }
    }

    pub fn new_interface(name: PooledStr, descriptor: &PooledStr) -> Option<Self> {
        let method_type = MethodTypeSymbol::new(descriptor)?;
        Some(Self {
            name,
            descriptor: method_type,
        })
    }

    pub fn not_initialization(&self) -> bool {
        (&*self.name) != "<init>" && (&*self.name) != "<clinit>"
    }
}
pub fn check_is_unqualified_name(s: &str) -> bool {
    for b in s.chars() {
        match b as char {
            '.' | ';' | '[' | '/' => {
                return false;
            }
            _ => {}
        }
    }
    true
}
pub fn check_is_field_name(s: &str) -> bool {
    for b in s.chars() {
        match b as char {
            '.' | ';' | '[' | '/' => {
                return false;
            }
            _ => {}
        }
    }
    true
}
pub fn check_is_method_name(s: &str) -> bool {
    if s != "<init>" && s != "<clinit>" {
        for b in s.bytes() {
            match b as char {
                '.' | ';' | '[' | '/' => {
                    return false;
                }
                _ => {}
            }
        }
    }
    true
}
pub fn check_is_class_name(s: &str) -> bool {
    let iter = s.bytes();
    for c in iter {
        match c {
            b'.' => {
                return false;
            }
            _ => {}
        }
    }
    true
}
pub fn check_is_type_name(s: &str) -> bool {
    let mut iter = s.bytes().peekable();
    if s.is_empty() {
        return false;
    }
    let mut dimensions = 0;
    while let Some(c) = iter.peek() {
        match c {
            b'[' => {
                iter.next();
                if dimensions == 255 {
                    return false;
                }
                dimensions += 1;
            }
            _ => {
                break;
            }
        }
    }
    let c = match iter.next() {
        Some(c) => c,
        None => return false,
    };
    match c {
        b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' => {
            return true;
        }
        b'L' => {}
        _ => {
            return false;
        }
    }
    for c in iter.by_ref() {
        if c == b';' {
            break;
        }
    }
    if iter.next().is_some() {
        return false;
    }
    true
}
pub fn parser_type_name<'a, I: Iterator<Item = &'a u8>>(iter: &mut I) -> Option<String> {
    let mut s = Vec::<u8>::new();
    let mut dimensions = 0;
    let next_char;
    loop {
        if let Some(c) = iter.next() {
            match c {
                b'[' => {
                    if dimensions == 255 {
                        return None;
                    }
                    s.push(b'[');
                    dimensions += 1;
                }
                _ => {
                    next_char = *c;
                    break;
                }
            }
        } else {
            return None;
        }
    }
    let c = next_char;
    match c as char {
        'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => {
            s.push(c);
            return String::from_utf8(s).ok();
        }
        'L' => {
            s.push(b'L');
        }
        _ => {}
    }
    for c in iter {
        s.push(*c);
        if *c == b';' {
            break;
        }
    }
    if s.is_empty() {
        return None;
    }
    String::from_utf8(s).ok()
}
pub fn check_is_field_descriptor(s: &str) -> bool {
    check_is_type_name(s)
}
pub fn check_is_method_descriptor(s: &str) -> bool {
    let mut iter = s.as_bytes().iter().peekable();
    if iter.next() != Some(&b'(') {
        return false;
    };
    while iter.peek() != Some(&&b')') {
        if parser_type_name(&mut iter).is_none() {
            return false;
        }
    }
    if iter.next() != Some(&b')') {
        return false;
    };
    if iter.peek() != Some(&&b'V') || iter.len() != 1 {
        if let Some(return_type_name) = parser_type_name(&mut iter) {
            TypeSymbol::new_unchecked(return_type_name.into())
        } else {
            return false;
        };
        if iter.next().is_some() {
            return false;
        };
    };
    true
}
pub fn check_is_initialization_method_descriptor(s: &str) -> bool {
    let mut iter = s.as_bytes().iter().peekable();
    if iter.next() != Some(&b'(') {
        return false;
    };
    while iter.peek() != Some(&&b')') {
        if parser_type_name(&mut iter).is_none() {
            return false;
        }
    }
    if iter.next() != Some(&b')') {
        return false;
    };

    if iter.peek() != Some(&&b'V') || iter.len() != 1 {
        return false;
    };
    true
}
