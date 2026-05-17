//! # Intermediate Representation (IR)
//!
//! A language-neutral AST that captures:
//! - Types (with ownership and lifetime hints)
//! - Control flow
//! - Functions, structs, and traits/classes

use std::fmt;

/// A complete IR module (single file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub items: Vec<Item>,
}

/// Top-level item in a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Function(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Impl(ImplBlock),
    TypeAlias(String, Type),
    Use(String), // import/using
    TODOComment(String),
}

/// Function definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Parameter>,
    pub ret_ty: Option<Type>,
    pub body: Block,
    /// True if this function is marked `unsafe` (Rust) or uses raw pointers (C++).
    pub is_unsafe: bool,
    /// True if this function is marked `virtual` (C++) or is a trait method (Rust).
    pub is_virtual: bool,
    /// True if this is a method (associated with a type).
    pub is_method: bool,
    /// For methods: the `self` parameter style.
    pub self_param: Option<SelfParam>,
}

/// Self parameter style for methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfParam {
    Value,       // self / by-value
    Ref,         // &self / const T&
    MutRef,      // &mut self / T&
    Ptr,         // *const self / const T*
    MutPtr,      // *mut self / T*
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

/// Generic parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<Type>, // trait bounds / concepts
}

/// Block of statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>,
}

/// Statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let(LetStmt),
    Expr(Box<Expr>),
    Return(Option<Box<Expr>>),
    Assign(Box<Expr>, Box<Expr>), // lhs = rhs
    Block(Block),
}

/// Let binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetStmt {
    pub name: String,
    pub ty: Option<Type>,
    pub init: Option<Box<Expr>>,
    pub mutable: bool,
}

/// Expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    If(Box<Expr>, Block, Option<Block>),
    Match(Box<Expr>, Vec<Arm>),
    While(Box<Expr>, Block),
    For(Box<ForHead>, Block),
    Block(Block),
    StructInit(String, Vec<(String, Expr)>), // Struct { field: expr, ... }
    ArrayInit(Vec<Expr>),
    Tuple(Vec<Expr>),
    Break,
    Continue,
    /// Lambda / closure expression.
    Closure(Vec<Parameter>, Option<Type>, Block),
    /// A placeholder for unsupported expressions.
    TODO(String),
}

/// Match arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arm {
    pub pat: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
}

/// Pattern in match arms / let bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Wild,                     // _
    Literal(Literal),
    Ident(String),
    Struct(String, Vec<(String, Pattern)>),
    Tuple(Vec<Pattern>),
    Array(Vec<Pattern>),
    Ref(Box<Pattern>),
    Mut(Box<Pattern>),
    Or(Vec<Pattern>),
}

/// For loop header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForHead {
    pub pat: Pattern,
    pub expr: Box<Expr>,
}

/// Binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    And, Or,
    Eq, Ne, Lt, Le, Gt, Ge,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Assign, AddAssign, SubAssign, MulAssign, DivAssign, RemAssign,
}

/// Unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg, Not, Deref, Ref, RefMut,
}

/// Literal value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Int(String),    // preserve base/radix as string
    Float(String),
    String(String),
    Char(char),
    Bool(bool),
}

/// Type representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unit,                        // () / void
    Never,                       // !
    Infer,                       // _ / auto
    Named(String, Vec<Type>),    // Vec<T>, std::vector<T>
    Ref(Box<Type>, Mutability),  // &T, &mut T, const T&, T&
    Ptr(Box<Type>, Mutability),  // *const T, *mut T, T*, const T*
    Array(Box<Type>, Option<usize>),
    Tuple(Vec<Type>),
    Function(Vec<Type>, Box<Type>), // fn(args) -> ret
    Slice(Box<Type>),            // [T]
    /// Placeholder for unsupported types.
    TODO(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Mut,
    Not,
}

/// Base class with access specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseClass {
    pub ty: Type,
    pub visibility: Visibility,
    pub is_virtual: bool,
}

/// Struct definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDef {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<FieldDef>,
    pub is_class: bool, // true for C++ class, false for struct
    pub methods: Vec<Function>,
    pub base_classes: Vec<BaseClass>,
}

/// Field definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub ty: Type,
    pub visibility: Visibility,
}

/// Enum definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<VariantDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantDef {
    pub name: String,
    pub fields: VariantFields,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantFields {
    Unit,
    Tuple(Vec<Type>),
    Struct(Vec<FieldDef>),
}

/// Trait / concept definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDef {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub items: Vec<TraitItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraitItem {
    Function(Function),
    Type(String, Vec<Type>), // associated type with bounds
    Const(String, Type, Option<Literal>),
    TODOComment(String),
}

/// impl block / class method implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplBlock {
    pub generics: Vec<GenericParam>,
    pub trait_name: Option<String>, // None for inherent impl
    pub for_type: Type,
    pub items: Vec<ImplItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImplItem {
    Function(Function),
    Const(String, Type, Option<Literal>),
    Type(String, Type),
}

/// Visibility qualifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Assign => "=",
            BinOp::AddAssign => "+=",
            BinOp::SubAssign => "-=",
            BinOp::MulAssign => "*=",
            BinOp::DivAssign => "/=",
            BinOp::RemAssign => "%=",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
            UnOp::Deref => "*",
            UnOp::Ref => "&",
            UnOp::RefMut => "&mut ",
        };
        write!(f, "{}", s)
    }
}
