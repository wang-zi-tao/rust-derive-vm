// use std::{
//     alloc::Layout,
//     collections::HashSet,
//     convert::TryInto,
//     hash::{Hash, Hasher},
//     ops::Range,
//     sync::Arc,
// };
//
// use failure::{format_err, Fallible};
// use vm_core::{FloatKind, FunctionType, IntKind, ReferenceKind, Type, TypeResource};
//
// use crate::{graph::RegistedType, heap::HEAP_SEGMENT_SIZE, metadata::Metadata};
//
// #[derive(Debug)]
// pub struct TypeLayout {
//     base_size: u32,
//     variable_size: u32,
//     tire: u32,
//     align: u32,
//     type_layout_kind: TypeLayoutKind,
//     metadata_layout: Vec<Metadata>,
// }
// impl TypeLayout {
//     pub fn size(&self) -> u32 {
//         self.base_size
//     }
//
//     pub fn tire(&self) -> u32 {
//         self.tire
//     }
//
//     pub fn type_layout_kind(&self) -> &TypeLayoutKind {
//         &self.type_layout_kind
//     }
// }
// impl TypeLayout {
//     pub fn arrange(input: Type) -> Fallible<TypeLayout> {
//         let type_layout = match input {
//             Type::Float(FloatKind::F32) => TypeLayout {
//                 base_size: 4,
//                 variable_size: 0,
//                 tire: 0,
//                 align: 4,
//                 type_layout_kind: TypeLayoutKind::Float(FloatKind::F32),
//                 metadata_layout: Vec::new(),
//             },
//             Type::Float(FloatKind::F64) => TypeLayout {
//                 base_size: 8,
//                 variable_size: 0,
//                 tire: 0,
//                 align: 8,
//                 type_layout_kind: TypeLayoutKind::Float(FloatKind::F64),
//                 metadata_layout: Vec::new(),
//             },
//             Type::Int(kind) => TypeLayout {
//                 base_size: kind.get_layout().size().try_into()?,
//                 variable_size: 0,
//                 tire: 0,
//                 align: { kind.get_layout().align().try_into()? },
//                 type_layout_kind: TypeLayoutKind::Int(kind),
//                 metadata_layout: Vec::new(),
//             },
//             Type::Native(layout) => TypeLayout {
//                 base_size: layout.size().try_into()?,
//                 variable_size: 0,
//                 tire: 0,
//                 align: layout.align().try_into()?,
//                 type_layout_kind: TypeLayoutKind::Native(layout),
//                 metadata_layout: Vec::new(),
//             },
//             Type::Array(value_type, size) => {
//                 let element_layout = TypeLayout::arrange(*value_type)?;
//                 if element_layout.variable_size != 0 {
//                     Err(format_err!(
//                         "unsized type can not use as a element of array"
//                     ))?;
//                 }
//                 TypeLayout {
//                     base_size: size.map(|s| s as u32).unwrap_or(8) * element_layout.base_size,
//                     variable_size: if size.is_some() {
//                         0
//                     } else {
//                         Layout::from_size_align(element_layout.base_size, element_layout.align)?
//                             .pad_to_align()
//                             .size()
//                     },
//                     tire: element_layout.tire,
//                     align: if size.is_none() {
//                         element_layout.align
//                     } else {
//                         u32::max(element_layout.align, 8)
//                     },
//                     type_layout_kind: TypeLayoutKind::Array(
//                         Box::new(element_layout.type_layout_kind),
//                         size,
//                     ),
//                     metadata_layout: element_layout.metadata_layout,
//                 }
//             }
//             Type::Pointer(layout) => {
//                 let pointer_layout = Layout::new::<*const ()>();
//                 TypeLayout {
//                     base_size: pointer_layout.size() as u32,
//                     variable_size: 0,
//                     tire: 0,
//                     align: pointer_layout.align() as u32,
//                     type_layout_kind: TypeLayoutKind::Pointer(Box::new(TypeLayout::arrange(
//                         *layout,
//                     )?)),
//                     metadata_layout: Vec::new(),
//                 }
//             }
//             Type::Function(layout) => {
//                 let pointer_layout = Layout::new::<*const ()>();
//                 TypeLayout {
//                     base_size: pointer_layout.size() as u32,
//                     variable_size: 0,
//                     tire: 0,
//                     align: pointer_layout.align() as u32,
//                     type_layout_kind: TypeLayoutKind::Function(layout),
//                     metadata_layout: Vec::new(),
//                 }
//             }
//             Type::MetaData(metadata) => {
//                 let mut metadata_layout = Vec::with_capacity(metadata.len());
//                 let tire = metadata.len().try_into()?;
//                 for (oop, value_type) in Vec::from(metadata) {
//                     if let Some(value_type) = value_type {
//                         metadata_layout.push(Metadata::new(
//                             Some(RegistedType::from_dyn_arc(value_type)?),
//                             oop,
//                         ));
//                     } else {
//                         metadata_layout.push(Metadata::new(None, oop));
//                     }
//                 }
//                 TypeLayout {
//                     base_size: 0,
//                     variable_size: 0,
//                     tire,
//                     align: 1,
//                     type_layout_kind: TypeLayoutKind::Tuple(vec![].into()),
//                     metadata_layout,
//                 }
//             }
//             Type::Reference(value_type, reference_kind) => {
//                 Self::arrange_reference_layout(value_type, reference_kind)?
//             }
//             Type::Embed(value_type) => {
//                 let registered_type = RegistedType::from_dyn_arc(value_type)?;
//                 let inner = registered_type.get_type_layout()?;
//                 TypeLayout {
//                     base_size: inner.base_size,
//                     variable_size: inner.variable_size,
//                     tire: inner.tire,
//                     align: inner.align,
//                     metadata_layout: inner.metadata_layout.clone(),
//                     type_layout_kind: TypeLayoutKind::Embed(registered_type),
//                 }
//             }
//             Type::Const(value, value_type) => {
//                 let inner = TypeLayout::arrange(*value_type)?;
//                 TypeLayout {
//                     base_size: inner.base_size,
//                     variable_size: inner.variable_size,
//                     align: inner.align,
//                     tire: inner.tire,
//                     metadata_layout: inner.metadata_layout.clone(),
//                     type_layout_kind: TypeLayoutKind::Const(value, Box::new(inner)),
//                 }
//             }
//             Type::Tuple(fields) => {
//                 let mut layout = Layout::new::<()>();
//                 let mut arranged_fields = Vec::with_capacity(fields.len());
//                 let mut tire = 0;
//                 let mut metadata_layout = Vec::new();
//                 let mut iter = Vec::from(fields).into_iter();
//                 let mut variable_size = 0;
//                 while let Some(field) = iter.next() {
//                     let field_layout = Self::arrange(field)?;
//                     let (new_layout, offset) = layout.extend(field_layout.get_layout()?)?;
//                     let new_tire = field_layout.tire;
//                     metadata_layout.extend(field_layout.metadata_layout.iter().cloned());
//                     if iter.len() != 0 {
//                         if field_layout.variable_size != 0 {
//                             Err(format_err!(
//                                 "unsized type can be only plased at the end of the type"
//                             ))?;
//                         }
//                     } else {
//                         variable_size = field_layout.variable_size;
//                     }
//                     arranged_fields.push(EmbedTypeLaout {
//                         embed_type: field_layout.type_layout_kind,
//                         offset: Self::arrange_offset(offset.try_into()?, tire),
//                     });
//                     tire += new_tire;
//                     layout = new_layout;
//                 }
//                 TypeLayout {
//                     base_size: layout.size() as u32,
//                     variable_size,
//                     align: layout.align() as u32,
//                     tire,
//                     metadata_layout,
//                     type_layout_kind: TypeLayoutKind::Tuple(arranged_fields.into()),
//                 }
//             }
//             Type::Union(fields) => {
//                 let mut size = 0;
//                 let mut align = 0;
//                 let mut arranged_fields = Vec::with_capacity(fields.len());
//                 let mut tire = 0;
//                 let mut metadata_layout = Vec::new();
//                 for field in Vec::from(fields) {
//                     let field_layout = Self::arrange(field.0)?;
//                     size = u32::max(size, field_layout.base_size);
//                     align = u32::max(align, field_layout.align);
//                     if field_layout.variable_size != 0 {
//                         Err(format_err!("unsized type can not be placed in the union"))?;
//                     }
//                     let new_tire = field_layout.tire;
//                     metadata_layout.extend(field_layout.metadata_layout.iter().cloned());
//                     arranged_fields.push(EmbedTypeLaout {
//                         embed_type: field_layout.type_layout_kind,
//                         offset: Self::arrange_offset(0, tire),
//                     });
//                     tire += new_tire;
//                 }
//                 TypeLayout {
//                     base_size: size,
//                     variable_size: 0,
//                     align,
//                     tire,
//                     metadata_layout,
//                     type_layout_kind: TypeLayoutKind::Union(arranged_fields.into()),
//                 }
//             }
//             Type::Enum(cases) => {
//                 let case_count = cases.len();
//                 let (tag_size, tag_align) = match case_count {
//                     0..=0xff => (1, 1),
//                     0x100..=0xffff => (2, 2),
//                     0x1_0000..=0xffff_ffff => (4, 4),
//                     _ => (8, 8),
//                 };
//                 let tag_layout = Layout::from_size_align(tag_size as usize, tag_align as usize)?;
//                 let mut size = 0;
//                 let mut align = tag_align;
//                 let mut arranged_cases = Vec::with_capacity(cases.len());
//                 let mut tire = 0;
//                 let mut metadata_layout = Vec::new();
//                 for field in Vec::from(cases) {
//                     let field_layout = Self::arrange(field.0)?;
//                     size = u32::max(size, field_layout.base_size);
//                     align = u32::max(align, field_layout.align);
//                     if field_layout.variable_size != 0 {
//                         Err(format_err!("unsized type can not be placed in the enum"))?;
//                     }
//                     let new_tire = field_layout.tire;
//                     metadata_layout.extend(field_layout.metadata_layout.iter().cloned());
//                     arranged_cases.push(EmbedTypeLaout {
//                         embed_type: field_layout.type_layout_kind,
//                         offset: Self::arrange_offset(0, tire),
//                     });
//                     tire += new_tire;
//                 }
//                 let (new_layout, _offset) =
//                     tag_layout.extend(Layout::from_size_align(size as usize, align as usize)?)?;
//                 TypeLayout {
//                     base_size: new_layout.size().try_into()?,
//                     variable_size: 0,
//                     align: new_layout.align().try_into()?,
//                     tire,
//                     metadata_layout,
//                     type_layout_kind: TypeLayoutKind::Enum(arranged_cases.into()),
//                 }
//             }
//         };
//         Ok(type_layout)
//     }
//
//     fn arrange_offset(offset: u32, tire: u32) -> usize {
//         offset as usize + tire as usize * HEAP_SEGMENT_SIZE
//     }
//
//     fn arrange_reference_layout(
//         value_type: Arc<dyn TypeResource>,
//         reference_kind: Arc<dyn vm_core::ReferenceKind>,
//     ) -> Fallible<Self> {
//         Ok(TypeLayout {
//             base_size: reference_kind.size(),
//             variable_size: 0,
//             tire: 0,
//             align: reference_kind.align(),
//             type_layout_kind: TypeLayoutKind::Reference(ReferenceLayout {
//                 target_type: RegistedType::from_dyn_arc(value_type)?,
//                 reference_kind,
//             }),
//             metadata_layout: Vec::new(),
//         })
//     }
//
//     pub fn get_layout(&self) -> Fallible<Layout> {
//         Layout::from_size_align(self.base_size as usize, self.align as usize).map_err(|e| e.into())
//     }
// }
// impl Hash for TypeLayout {
//     fn hash<H>(&self, h: &mut H)
//     where
//         H: Hasher,
//     {
//         (self as *const TypeLayout).hash(h)
//     }
// }
// impl PartialEq for TypeLayout {
//     fn eq(&self, other: &Self) -> bool {
//         (self as *const TypeLayout) == (other as *const TypeLayout)
//     }
// }
// impl Eq for TypeLayout {}
// #[derive(Debug)]
// pub struct EmbedTypeLaout {
//     pub embed_type: TypeLayoutKind,
//     pub offset: usize,
// }
// #[derive(Debug)]
// pub struct ReferenceLayout {
//     pub target_type: Arc<RegistedType>,
//     pub reference_kind: Arc<dyn ReferenceKind>,
// }
// #[derive(Debug)]
// pub enum TypeLayoutKind {
//     Float(FloatKind),
//     Int(IntKind),
//     Const(Box<Value>, Box<TypeLayout>),
//     Tuple(Box<[EmbedTypeLaout]>),
//     Enum(Box<[EmbedTypeLaout]>),
//     Union(Box<[EmbedTypeLaout]>),
//     Pointer(Box<TypeLayout>),
//     Array(Box<TypeLayoutKind>, Option<usize>),
//     Reference(ReferenceLayout),
//     ReferenceOrEmbed(ReferenceLayout, usize),
//     Embed(Arc<RegistedType>),
//     Native(Layout),
//     Function(Arc<FunctionType>),
// }
// impl TypeLayoutKind {
//     pub(crate) fn for_each_reference(
//         &self,
//         mut f: impl FnMut(&ReferenceLayout) -> Fallible<()>,
//     ) -> Fallible<()> {
//         let mut work_stack = Vec::new();
//         work_stack.push(self);
//         while let Some(last) = work_stack.pop() {
//             match last {
//                 TypeLayoutKind::Tuple(t) | TypeLayoutKind::Enum(t) | TypeLayoutKind::Union(t) => {
//                     work_stack.extend(
//                         t.iter()
//                             .map(|embed_type_layout| &embed_type_layout.embed_type),
//                     );
//                 }
//                 TypeLayoutKind::Array(layout, _) => {
//                     work_stack.push(&**layout);
//                 }
//                 TypeLayoutKind::Reference(r, ..) | TypeLayoutKind::ReferenceOrEmbed(r, ..) => {
//                     f(r)?;
//                 }
//                 _ => {}
//             }
//         }
//         Ok(())
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use std::{alloc::Layout, collections::HashSet};
//
//     use super::TypeLayout;
//     use failure::Fallible;
//     use vm_core::{FloatKind, IntKind, Type};
//     #[test]
//     fn arrange() -> Fallible<()> {
//         let layout = TypeLayout::arrange(Type::Tuple(
//             [
//                 Type::Int(IntKind::U64),
//                 Type::Float(FloatKind::F32),
//                 Type::Float(FloatKind::F64),
//                 Type::Array(Box::new(Type::Int(IntKind::U8)), Some(8)),
//                 Type::Enum(
//                     [
//                         (Type::Int(IntKind::U8), 1, 1),
//                         (Type::Int(IntKind::Bool), 1, 1),
//                     ]
//                     .into(),
//                 ),
//                 Type::Union(
//                     [
//                         (Type::Int(IntKind::U8), 1, 1),
//                         (Type::Int(IntKind::Bool), 1, 1),
//                     ]
//                     .into(),
//                 ),
//                 Type::Native(Layout::new::<usize>()),
//             ]
//             .into(),
//         ))?;
//         assert_eq!(layout.base_size, 64);
//         assert_eq!(layout.align, 64);
//         assert_eq!(layout.variable_size, 0);
//         assert_eq!(layout.tire, 0);
//         assert_eq!(layout.metadata_layout.len(), layout.tire as usize);
//         Ok(())
//     }
// }
// struct FieldLayout {
//     offset: usize,
// }
// struct EnumLayout {
//     tag_offset_bit: usize,
//     tag_start: usize,
//     default_variant: usize,
// }
// enum UndefinedArea {
//     Bits(Range<usize>),
//     Numbers(Range<usize>),
// }
