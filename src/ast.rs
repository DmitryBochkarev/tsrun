//! Abstract Syntax Tree types for TypeScript

use crate::lexer::Span;

/// A complete program (script or module)
#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Statement>,
    pub source_type: SourceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Script,
    Module,
}

// ============ STATEMENTS ============

#[derive(Debug, Clone)]
pub enum Statement {
    // Declarations
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    ClassDeclaration(ClassDeclaration),

    // TypeScript (no-op at runtime)
    TypeAlias(TypeAliasDeclaration),
    InterfaceDeclaration(InterfaceDeclaration),
    EnumDeclaration(EnumDeclaration),

    // Control Flow
    Block(BlockStatement),
    If(IfStatement),
    Switch(SwitchStatement),
    For(ForStatement),
    ForIn(ForInStatement),
    ForOf(ForOfStatement),
    While(WhileStatement),
    DoWhile(DoWhileStatement),
    Try(TryStatement),

    // Jump
    Return(ReturnStatement),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Throw(ThrowStatement),

    // Module
    Import(ImportDeclaration),
    Export(ExportDeclaration),

    // Other
    Expression(ExpressionStatement),
    Empty,
    Debugger,
    Labeled(LabeledStatement),
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub expression: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BlockStatement {
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub kind: VariableKind,
    pub declarations: Vec<VariableDeclarator>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    Let,
    Const,
    Var,
}

#[derive(Debug, Clone)]
pub struct VariableDeclarator {
    pub id: Pattern,
    pub type_annotation: Option<TypeAnnotation>,
    pub init: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    pub id: Option<Identifier>,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<TypeAnnotation>,
    pub type_parameters: Option<TypeParameters>,
    pub body: BlockStatement,
    pub generator: bool,
    pub async_: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub pattern: Pattern,
    pub type_annotation: Option<TypeAnnotation>,
    pub optional: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassDeclaration {
    pub id: Option<Identifier>,
    pub type_parameters: Option<TypeParameters>,
    pub super_class: Option<Box<Expression>>,
    pub implements: Vec<TypeReference>,
    pub body: ClassBody,
    pub decorators: Vec<Decorator>,
    pub abstract_: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassBody {
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ClassMember {
    Method(ClassMethod),
    Property(ClassProperty),
    Constructor(ClassConstructor),
    StaticBlock(BlockStatement),
}

#[derive(Debug, Clone)]
pub struct ClassMethod {
    pub key: ObjectPropertyKey,
    pub value: FunctionExpression,
    pub kind: MethodKind,
    pub computed: bool,
    pub static_: bool,
    pub accessibility: Option<Accessibility>,
    pub decorators: Vec<Decorator>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    Method,
    Get,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accessibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone)]
pub struct ClassProperty {
    pub key: ObjectPropertyKey,
    pub value: Option<Expression>,
    pub type_annotation: Option<TypeAnnotation>,
    pub computed: bool,
    pub static_: bool,
    pub readonly: bool,
    pub optional: bool,
    pub accessibility: Option<Accessibility>,
    pub decorators: Vec<Decorator>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassConstructor {
    pub params: Vec<FunctionParam>,
    pub body: BlockStatement,
    pub accessibility: Option<Accessibility>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Decorator {
    pub expression: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub test: Expression,
    pub consequent: Box<Statement>,
    pub alternate: Option<Box<Statement>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SwitchStatement {
    pub discriminant: Expression,
    pub cases: Vec<SwitchCase>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub test: Option<Expression>, // None for default
    pub consequent: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForStatement {
    pub init: Option<ForInit>,
    pub test: Option<Expression>,
    pub update: Option<Expression>,
    pub body: Box<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ForInit {
    Variable(VariableDeclaration),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct ForInStatement {
    pub left: ForInOfLeft,
    pub right: Expression,
    pub body: Box<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForOfStatement {
    pub left: ForInOfLeft,
    pub right: Expression,
    pub body: Box<Statement>,
    pub await_: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ForInOfLeft {
    Variable(VariableDeclaration),
    Pattern(Pattern),
}

#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub test: Expression,
    pub body: Box<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct DoWhileStatement {
    pub body: Box<Statement>,
    pub test: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TryStatement {
    pub block: BlockStatement,
    pub handler: Option<CatchClause>,
    pub finalizer: Option<BlockStatement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param: Option<Pattern>,
    pub body: BlockStatement,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub argument: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BreakStatement {
    pub label: Option<Identifier>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ContinueStatement {
    pub label: Option<Identifier>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ThrowStatement {
    pub argument: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LabeledStatement {
    pub label: Identifier,
    pub body: Box<Statement>,
    pub span: Span,
}

// TypeScript declarations

#[derive(Debug, Clone)]
pub struct TypeAliasDeclaration {
    pub id: Identifier,
    pub type_parameters: Option<TypeParameters>,
    pub type_annotation: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceDeclaration {
    pub id: Identifier,
    pub type_parameters: Option<TypeParameters>,
    pub extends: Vec<TypeReference>,
    pub body: Vec<TypeMember>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDeclaration {
    pub id: Identifier,
    pub members: Vec<EnumMember>,
    pub const_: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumMember {
    pub id: Identifier,
    pub initializer: Option<Expression>,
    pub span: Span,
}

// Module declarations

#[derive(Debug, Clone)]
pub struct ImportDeclaration {
    pub specifiers: Vec<ImportSpecifier>,
    pub source: StringLiteral,
    pub type_only: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ImportSpecifier {
    Named {
        local: Identifier,
        imported: Identifier,
        span: Span,
    },
    Default {
        local: Identifier,
        span: Span,
    },
    Namespace {
        local: Identifier,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct ExportDeclaration {
    pub declaration: Option<Box<Statement>>,
    pub specifiers: Vec<ExportSpecifier>,
    pub source: Option<StringLiteral>,
    pub default: bool,
    pub type_only: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExportSpecifier {
    pub local: Identifier,
    pub exported: Identifier,
    pub span: Span,
}

// ============ EXPRESSIONS ============

#[derive(Debug, Clone)]
pub enum Expression {
    // Literals
    Literal(Literal),
    Array(ArrayExpression),
    Object(ObjectExpression),
    Function(FunctionExpression),
    ArrowFunction(ArrowFunctionExpression),
    Class(ClassExpression),
    Template(TemplateLiteral),
    TaggedTemplate(TaggedTemplateExpression),

    // Identifiers
    Identifier(Identifier),
    This(Span),
    Super(Span),

    // Operations
    Unary(UnaryExpression),
    Binary(BinaryExpression),
    Logical(LogicalExpression),
    Conditional(ConditionalExpression),
    Assignment(AssignmentExpression),
    Update(UpdateExpression),
    Sequence(SequenceExpression),

    // Access
    Member(MemberExpression),
    OptionalChain(OptionalChainExpression),
    Call(CallExpression),
    New(NewExpression),

    // TypeScript
    TypeAssertion(TypeAssertionExpression),
    NonNull(NonNullExpression),

    // Special
    Spread(SpreadElement),
    Yield(YieldExpression),
    Await(AwaitExpression),

    // Parenthesized (for preserving source structure)
    Parenthesized(Box<Expression>, Span),
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(l) => l.span,
            Expression::Array(a) => a.span,
            Expression::Object(o) => o.span,
            Expression::Function(f) => f.span,
            Expression::ArrowFunction(a) => a.span,
            Expression::Class(c) => c.span,
            Expression::Template(t) => t.span,
            Expression::TaggedTemplate(t) => t.span,
            Expression::Identifier(i) => i.span,
            Expression::This(s) | Expression::Super(s) => *s,
            Expression::Unary(u) => u.span,
            Expression::Binary(b) => b.span,
            Expression::Logical(l) => l.span,
            Expression::Conditional(c) => c.span,
            Expression::Assignment(a) => a.span,
            Expression::Update(u) => u.span,
            Expression::Sequence(s) => s.span,
            Expression::Member(m) => m.span,
            Expression::OptionalChain(o) => o.span,
            Expression::Call(c) => c.span,
            Expression::New(n) => n.span,
            Expression::TypeAssertion(t) => t.span,
            Expression::NonNull(n) => n.span,
            Expression::Spread(s) => s.span,
            Expression::Yield(y) => y.span,
            Expression::Await(a) => a.span,
            Expression::Parenthesized(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub value: LiteralValue,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Null,
    Undefined,
    Boolean(bool),
    Number(f64),
    String(String),
    BigInt(String), // Store as string to preserve arbitrary precision
    RegExp { pattern: String, flags: String },
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrayExpression {
    pub elements: Vec<Option<ArrayElement>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Expression(Expression),
    Spread(SpreadElement),
}

#[derive(Debug, Clone)]
pub struct ObjectExpression {
    pub properties: Vec<ObjectProperty>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ObjectProperty {
    Property(Property),
    Spread(SpreadElement),
}

#[derive(Debug, Clone)]
pub struct Property {
    pub key: ObjectPropertyKey,
    pub value: Expression,
    pub kind: PropertyKind,
    pub computed: bool,
    pub shorthand: bool,
    pub method: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ObjectPropertyKey {
    Identifier(Identifier),
    String(StringLiteral),
    Number(Literal),
    Computed(Box<Expression>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    Init,
    Get,
    Set,
}

#[derive(Debug, Clone)]
pub struct FunctionExpression {
    pub id: Option<Identifier>,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<TypeAnnotation>,
    pub type_parameters: Option<TypeParameters>,
    pub body: BlockStatement,
    pub generator: bool,
    pub async_: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrowFunctionExpression {
    pub params: Vec<FunctionParam>,
    pub return_type: Option<TypeAnnotation>,
    pub type_parameters: Option<TypeParameters>,
    pub body: ArrowFunctionBody,
    pub async_: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ArrowFunctionBody {
    Expression(Box<Expression>),
    Block(BlockStatement),
}

#[derive(Debug, Clone)]
pub struct ClassExpression {
    pub id: Option<Identifier>,
    pub type_parameters: Option<TypeParameters>,
    pub super_class: Option<Box<Expression>>,
    pub implements: Vec<TypeReference>,
    pub body: ClassBody,
    pub decorators: Vec<Decorator>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TemplateLiteral {
    pub quasis: Vec<TemplateElement>,
    pub expressions: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TemplateElement {
    pub value: String,
    pub tail: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TaggedTemplateExpression {
    pub tag: Box<Expression>,
    pub quasi: TemplateLiteral,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub operator: UnaryOp,
    pub argument: Box<Expression>,
    pub prefix: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Minus,      // -
    Plus,       // +
    Not,        // !
    BitNot,     // ~
    Typeof,     // typeof
    Void,       // void
    Delete,     // delete
}

#[derive(Debug, Clone)]
pub struct BinaryExpression {
    pub operator: BinaryOp,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,        // +
    Sub,        // -
    Mul,        // *
    Div,        // /
    Mod,        // %
    Exp,        // **

    // Comparison
    Eq,         // ==
    NotEq,      // !=
    StrictEq,   // ===
    StrictNotEq, // !==
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=

    // Bitwise
    BitAnd,     // &
    BitOr,      // |
    BitXor,     // ^
    LShift,     // <<
    RShift,     // >>
    URShift,    // >>>

    // Other
    In,         // in
    Instanceof, // instanceof
}

#[derive(Debug, Clone)]
pub struct LogicalExpression {
    pub operator: LogicalOp,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,        // &&
    Or,         // ||
    NullishCoalescing, // ??
}

#[derive(Debug, Clone)]
pub struct ConditionalExpression {
    pub test: Box<Expression>,
    pub consequent: Box<Expression>,
    pub alternate: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignmentExpression {
    pub operator: AssignmentOp,
    pub left: AssignmentTarget,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AssignmentTarget {
    Identifier(Identifier),
    Member(MemberExpression),
    Pattern(Pattern),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentOp {
    Assign,     // =
    AddAssign,  // +=
    SubAssign,  // -=
    MulAssign,  // *=
    DivAssign,  // /=
    ModAssign,  // %=
    ExpAssign,  // **=
    BitAndAssign, // &=
    BitOrAssign,  // |=
    BitXorAssign, // ^=
    LShiftAssign, // <<=
    RShiftAssign, // >>=
    URShiftAssign, // >>>=
    AndAssign,  // &&=
    OrAssign,   // ||=
    NullishAssign, // ??=
}

#[derive(Debug, Clone)]
pub struct UpdateExpression {
    pub operator: UpdateOp,
    pub argument: Box<Expression>,
    pub prefix: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Increment,  // ++
    Decrement,  // --
}

#[derive(Debug, Clone)]
pub struct SequenceExpression {
    pub expressions: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MemberExpression {
    pub object: Box<Expression>,
    pub property: MemberProperty,
    pub computed: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MemberProperty {
    Identifier(Identifier),
    Expression(Box<Expression>),
    PrivateIdentifier(Identifier),
}

#[derive(Debug, Clone)]
pub struct OptionalChainExpression {
    pub base: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CallExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Argument>,
    pub type_arguments: Option<TypeArguments>,
    pub optional: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Argument {
    Expression(Expression),
    Spread(SpreadElement),
}

#[derive(Debug, Clone)]
pub struct NewExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Argument>,
    pub type_arguments: Option<TypeArguments>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SpreadElement {
    pub argument: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct YieldExpression {
    pub argument: Option<Box<Expression>>,
    pub delegate: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AwaitExpression {
    pub argument: Box<Expression>,
    pub span: Span,
}

// TypeScript expressions

#[derive(Debug, Clone)]
pub struct TypeAssertionExpression {
    pub expression: Box<Expression>,
    pub type_annotation: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct NonNullExpression {
    pub expression: Box<Expression>,
    pub span: Span,
}

// ============ PATTERNS ============

#[derive(Debug, Clone)]
pub enum Pattern {
    Identifier(Identifier),
    Object(ObjectPattern),
    Array(ArrayPattern),
    Rest(RestElement),
    Assignment(AssignmentPattern),
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Identifier(i) => i.span,
            Pattern::Object(o) => o.span,
            Pattern::Array(a) => a.span,
            Pattern::Rest(r) => r.span,
            Pattern::Assignment(a) => a.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectPattern {
    pub properties: Vec<ObjectPatternProperty>,
    pub type_annotation: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ObjectPatternProperty {
    KeyValue {
        key: ObjectPropertyKey,
        value: Pattern,
        shorthand: bool,
        span: Span,
    },
    Rest(RestElement),
}

#[derive(Debug, Clone)]
pub struct ArrayPattern {
    pub elements: Vec<Option<Pattern>>,
    pub type_annotation: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RestElement {
    pub argument: Box<Pattern>,
    pub type_annotation: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignmentPattern {
    pub left: Box<Pattern>,
    pub right: Box<Expression>,
    pub span: Span,
}

// ============ TYPE ANNOTATIONS ============

#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    Keyword(TypeKeyword),
    Reference(TypeReference),
    Literal(TypeLiteral),
    Object(ObjectType),
    Array(ArrayType),
    Tuple(TupleType),
    Union(UnionType),
    Intersection(IntersectionType),
    Function(FunctionType),
    Conditional(ConditionalType),
    Infer(InferType),
    Mapped(MappedType),
    Indexed(IndexedAccessType),
    Typeof(TypeofType),
    Keyof(KeyofType),
    Parenthesized(Box<TypeAnnotation>),
    This,
}

#[derive(Debug, Clone)]
pub struct TypeKeyword {
    pub keyword: TypeKeywordKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKeywordKind {
    Any,
    Unknown,
    Never,
    Void,
    Null,
    Undefined,
    Boolean,
    Number,
    String,
    Symbol,
    BigInt,
    Object,
}

#[derive(Debug, Clone)]
pub struct TypeReference {
    pub name: Identifier,
    pub type_arguments: Option<TypeArguments>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeArguments {
    pub params: Vec<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeParameters {
    pub params: Vec<TypeParameter>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeParameter {
    pub name: Identifier,
    pub constraint: Option<Box<TypeAnnotation>>,
    pub default: Option<Box<TypeAnnotation>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeLiteral {
    pub value: LiteralValue,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ObjectType {
    pub members: Vec<TypeMember>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeMember {
    Property(PropertySignature),
    Method(MethodSignature),
    Index(IndexSignature),
    Call(CallSignature),
    Construct(ConstructSignature),
}

#[derive(Debug, Clone)]
pub struct PropertySignature {
    pub key: ObjectPropertyKey,
    pub type_annotation: Option<TypeAnnotation>,
    pub optional: bool,
    pub readonly: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub key: ObjectPropertyKey,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<TypeAnnotation>,
    pub type_parameters: Option<TypeParameters>,
    pub optional: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IndexSignature {
    pub key: Identifier,
    pub key_type: TypeAnnotation,
    pub value_type: TypeAnnotation,
    pub readonly: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CallSignature {
    pub params: Vec<FunctionParam>,
    pub return_type: Option<TypeAnnotation>,
    pub type_parameters: Option<TypeParameters>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstructSignature {
    pub params: Vec<FunctionParam>,
    pub return_type: Option<TypeAnnotation>,
    pub type_parameters: Option<TypeParameters>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub element_type: Box<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TupleType {
    pub element_types: Vec<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnionType {
    pub types: Vec<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IntersectionType {
    pub types: Vec<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub params: Vec<FunctionParam>,
    pub return_type: Box<TypeAnnotation>,
    pub type_parameters: Option<TypeParameters>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConditionalType {
    pub check_type: Box<TypeAnnotation>,
    pub extends_type: Box<TypeAnnotation>,
    pub true_type: Box<TypeAnnotation>,
    pub false_type: Box<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InferType {
    pub type_parameter: TypeParameter,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MappedType {
    pub type_parameter: TypeParameter,
    pub name_type: Option<Box<TypeAnnotation>>,
    pub type_annotation: Option<Box<TypeAnnotation>>,
    pub readonly: Option<MappedTypeModifier>,
    pub optional: Option<MappedTypeModifier>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedTypeModifier {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub struct IndexedAccessType {
    pub object_type: Box<TypeAnnotation>,
    pub index_type: Box<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeofType {
    pub expression: Identifier,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct KeyofType {
    pub type_annotation: Box<TypeAnnotation>,
    pub span: Span,
}
