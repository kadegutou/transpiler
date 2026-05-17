//! # C++ Parser (tree-sitter-cpp)
//!
//! Parses C++17/20 source into the IR using tree-sitter.

use crate::ir::*;
use anyhow::{bail, Result};
use tree_sitter::Node;

/// Parse C++ source into the IR.
pub fn parse(source: &str) -> Result<Module> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse C++ source"))?;
    let root = tree.root_node();
    parse_translation_unit(root, source)
}

fn text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

/// Recursively extract the first identifier/field_identifier/type_identifier from a node.
fn extract_identifier_name(node: Node, source: &str) -> String {
    if node.kind() == "identifier" || node.kind() == "field_identifier" || node.kind() == "type_identifier" {
        return text(node, source).to_string();
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let name = extract_identifier_name(child, source);
        if !name.is_empty() {
            return name;
        }
    }
    String::new()
}

// =========================================================================
// Top level
// =========================================================================

fn parse_translation_unit(node: Node, source: &str) -> Result<Module> {
    let mut items = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Ok(f) = parse_function_definition(child, source) {
                    items.push(Item::Function(f));
                }
            }
            "declaration" => {
                if let Ok(decl_items) = parse_declaration(child, source) {
                    for item in decl_items {
                        match item {
                            Item::Function(ref f) if f.body.stmts.len() == 1 && matches!(f.body.stmts[0], Stmt::Let(_)) => {
                                if let Stmt::Let(ref l) = f.body.stmts[0] {
                                    items.push(Item::TODOComment(format!(
                                        "global variable: {} : {:?}",
                                        l.name, f.ret_ty
                                    )));
                                }
                            }
                            other => items.push(other),
                        }
                    }
                }
            }
            "class_specifier" => {
                if let Ok(s) = parse_class_specifier(child, source) {
                    items.push(Item::Struct(s));
                }
            }
            "enum_specifier" => {
                if let Ok(e) = parse_enum_specifier(child, source) {
                    items.push(Item::Enum(e));
                }
            }
            "template_declaration" => {
                match parse_template_declaration(child, source) {
                    Ok(template_items) => items.extend(template_items),
                    Err(_) => items.push(Item::TODOComment("template declaration".to_string())),
                }
            }
            "namespace_definition" => {
                match parse_namespace_definition(child, source) {
                    Ok(ns_items) => items.extend(ns_items),
                    Err(_) => items.push(Item::TODOComment("namespace".to_string())),
                }
            }
            "preproc_include" | "preproc_def" | "preproc_function_def" | "comment" => {}
            "using_declaration" => {
                if let Ok(use_item) = parse_using_declaration(child, source) {
                    items.push(use_item);
                }
            }
            "alias_declaration" | "type_alias_declaration" => {
                if let Ok(alias_item) = parse_alias_declaration(child, source) {
                    items.push(alias_item);
                }
            }
            ";" => {}
            _ => {
                items.push(Item::TODOComment(format!(
                    "unsupported top-level: {}",
                    child.kind()
                )));
            }
        }
    }
    Ok(Module { items })
}

/// Parse a template declaration: extract generics and attach to the inner declaration.
fn parse_template_declaration(node: Node, source: &str) -> Result<Vec<Item>> {
    let mut generics = Vec::new();
    let mut inner_items = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "template_parameter_list" => {
                generics = parse_template_params(child, source)?;
            }
            "function_definition" => {
                if let Ok(mut f) = parse_function_definition(child, source) {
                    f.generics.extend(generics.clone());
                    inner_items.push(Item::Function(f));
                }
            }
            "class_specifier" => {
                if let Ok(mut s) = parse_class_specifier(child, source) {
                    s.generics.extend(generics.clone());
                    inner_items.push(Item::Struct(s));
                }
            }
            "declaration" => {
                if let Ok(decl_items) = parse_declaration(child, source) {
                    for item in decl_items {
                        match item {
                            Item::Function(mut f) => {
                                f.generics.extend(generics.clone());
                                inner_items.push(Item::Function(f));
                            }
                            other => inner_items.push(other),
                        }
                    }
                }
            }
            "template" | "<" | ">" => {}
            _ => {
                // Try to parse as inner declaration
                if child.is_named() {
                    inner_items.push(Item::TODOComment(format!(
                        "template with unsupported inner: {}",
                        child.kind()
                    )));
                }
            }
        }
    }

    if inner_items.is_empty() {
        inner_items.push(Item::TODOComment("empty template declaration".to_string()));
    }

    Ok(inner_items)
}

/// Parse a namespace definition: flatten its contents into the parent scope.
fn parse_namespace_definition(node: Node, source: &str) -> Result<Vec<Item>> {
    let mut items = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "declaration_list" => {
                // Recurse into declaration_list, treating each item as top-level
                let mut c2 = child.walk();
                for ch in child.children(&mut c2) {
                    match ch.kind() {
                        "function_definition" => {
                            if let Ok(f) = parse_function_definition(ch, source) {
                                items.push(Item::Function(f));
                            }
                        }
                        "declaration" => {
                            if let Ok(decl_items) = parse_declaration(ch, source) {
                                for item in decl_items {
                                    match item {
                                        Item::Function(ref f) if f.body.stmts.len() == 1 && matches!(f.body.stmts[0], Stmt::Let(_)) => {
                                            if let Stmt::Let(ref l) = f.body.stmts[0] {
                                                items.push(Item::TODOComment(format!(
                                                    "global variable: {} : {:?}",
                                                    l.name, f.ret_ty
                                                )));
                                            }
                                        }
                                        other => items.push(other),
                                    }
                                }
                            }
                        }
                        "class_specifier" => {
                            if let Ok(s) = parse_class_specifier(ch, source) {
                                items.push(Item::Struct(s));
                            }
                        }
                        "enum_specifier" => {
                            if let Ok(e) = parse_enum_specifier(ch, source) {
                                items.push(Item::Enum(e));
                            }
                        }
                        "template_declaration" => {
                            match parse_template_declaration(ch, source) {
                                Ok(template_items) => items.extend(template_items),
                                Err(_) => items.push(Item::TODOComment("template declaration".to_string())),
                            }
                        }
                        "namespace_definition" => {
                            match parse_namespace_definition(ch, source) {
                                Ok(ns_items) => items.extend(ns_items),
                                Err(_) => items.push(Item::TODOComment("namespace".to_string())),
                            }
                        }
                        "preproc_include" | "preproc_def" | "preproc_function_def" | "comment" | "{" | "}" | ";" => {}
                        _ => {
                            items.push(Item::TODOComment(format!(
                                "unsupported in namespace: {}",
                                ch.kind()
                            )));
                        }
                    }
                }
            }
            "namespace" | "identifier" | "{" | "}" | ";" => {}
            _ => {
                // skip non-named children
            }
        }
    }

    Ok(items)
}

/// Parse `using_declaration` — `using namespace::name;` or `using std::cout;`
fn parse_using_declaration(node: Node, source: &str) -> Result<Item> {
    // Walk children to build the full path string
    let mut parts = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "using" | "namespace" | ";" => {}
            "qualified_identifier" | "identifier" | "type_identifier"
            | "scope_resolution" | "::" | "nested_name_specifier" => {
                parts.push(text(child, source).to_string());
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        return Ok(Item::TODOComment("empty using declaration".to_string()));
    }
    Ok(Item::Use(parts.join("")))
}

/// Parse `alias_declaration` — `using name = type;`
fn parse_alias_declaration(node: Node, source: &str) -> Result<Item> {
    let mut name = String::new();
    let mut ty = Type::Infer;

    let mut cursor = node.walk();
    let mut found_eq = false;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "using" | ";" => {}
            "=" => found_eq = true,
            "identifier" | "type_identifier" if !found_eq => {
                name = text(child, source).to_string();
            }
            "type" | "template_type" | "qualified_identifier" | "type_identifier"
            | "primitive_type" | "sized_type_specifier" if found_eq => {
                if let Ok(t) = parse_type_node(child, source) {
                    ty = t;
                }
            }
            _ if found_eq && child.is_named() => {
                // Try to parse as type even if the kind is unexpected
                if ty == Type::Infer {
                    if let Ok(t) = parse_type_node(child, source) {
                        ty = t;
                    }
                }
            }
            _ => {}
        }
    }

    if name.is_empty() {
        return Ok(Item::TODOComment("empty alias declaration".to_string()));
    }
    Ok(Item::TypeAlias(name, ty))
}

// =========================================================================
// Function definition
// =========================================================================

fn parse_function_definition(node: Node, source: &str) -> Result<Function> {
    let mut ret_ty = Some(Type::Unit);
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Block { stmts: vec![], expr: None };
    let mut generics = Vec::new();
    let mut is_unsafe = false;
    let mut is_virtual = false;

    if let Some(type_node) = node.child_by_field_name("type") {
        ret_ty = Some(parse_type_node(type_node, source)?);
    }

    if let Some(decl_node) = node.child_by_field_name("declarator") {
        let (n, p, u) = parse_function_declarator(decl_node, source)?;
        name = n;
        params = p;
        is_unsafe = u;
    }

    if let Some(body_node) = node.child_by_field_name("body") {
        body = parse_compound_statement(body_node, source)?;
    }

    // Walk children for template params, virtual specifier, etc.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "template_parameter_list" => {
                generics = parse_template_params(child, source)?;
            }
            "virtual_function_specifier" | "virtual" | "virtual_specifier" => {
                is_virtual = true;
            }
            "override_specifier" => {
                // `override` implies virtual in C++; mark as virtual
                is_virtual = true;
            }
            "virtual_function_definition" => {
                // tree-sitter-cpp may wrap virtual methods in this node
                if let Ok(f) = parse_function_definition(child, source) {
                    name = f.name;
                    params = f.params;
                    ret_ty = f.ret_ty;
                    body = f.body;
                    generics = f.generics;
                    is_unsafe = f.is_unsafe;
                    is_virtual = true;
                }
            }
            _ => {}
        }
    }

    Ok(Function {
        name,
        generics,
        params,
        ret_ty,
        body,
        is_unsafe,
        is_virtual,
        is_method: false,
        self_param: None,
    })
}

fn parse_function_declarator(node: Node, source: &str) -> Result<(String, Vec<Parameter>, bool)> {
    let mut name = String::new();
    let mut params = Vec::new();
    let mut is_unsafe = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "field_identifier" | "operator_name" | "destructor_name" => {
                name = text(child, source).to_string();
            }
            "qualified_identifier" => {
                name = text(child, source).to_string();
            }
            "parameter_list" => {
                params = parse_parameter_list(child, source)?;
            }
            "abstract_function_declarator" | "function_declarator" | "parenthesized_declarator" => {
                // nested declarator: e.g. pointer to function
                let (n, p, u) = parse_function_declarator(child, source)?;
                if !n.is_empty() {
                    name = n;
                }
                if !p.is_empty() {
                    params = p;
                }
                is_unsafe = is_unsafe || u;
            }
            "pointer_declarator" | "reference_declarator" => {
                // function returning pointer/reference – ignore for name extraction
            }
            "type_qualifier" => {}
            _ => {}
        }
    }

    Ok((name, params, is_unsafe))
}

fn parse_parameter_list(node: Node, source: &str) -> Result<Vec<Parameter>> {
    let mut params = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            if let Ok(p) = parse_parameter_declaration(child, source) {
                params.push(p);
            }
        }
    }
    Ok(params)
}

fn parse_parameter_declaration(node: Node, source: &str) -> Result<Parameter> {
    let mut ty = Type::Infer;
    let mut name = String::new();
    let mut type_node_opt: Option<Node> = None;

    if let Some(type_node) = node.child_by_field_name("type") {
        ty = parse_type_node(type_node, source)?;
        type_node_opt = Some(type_node);
    }

    if let Some(decl_node) = node.child_by_field_name("declarator") {
        name = extract_identifier_name(decl_node, source);
        ty = augment_type_with_declarator(ty.clone(), decl_node, source).unwrap_or(ty);
        if let Some(type_node) = type_node_opt {
            ty = apply_const_from_type_node(ty, type_node, source);
        }
    }

    // tree-sitter-cpp places top-level type_qualifiers (e.g. `const`) as direct
    // children of parameter_declaration, not inside the `type` field.
    let mut is_const = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_qualifier" && text(child, source) == "const" {
            is_const = true;
            break;
        }
    }
    if is_const {
        ty = match ty {
            Type::Ref(inner, _) => Type::Ref(inner, Mutability::Not),
            Type::Ptr(inner, _) => Type::Ptr(inner, Mutability::Not),
            other => other,
        };
    }

    if name.is_empty() {
        name = "_".to_string();
    }

    Ok(Parameter { name, ty })
}

fn parse_template_params(node: Node, source: &str) -> Result<Vec<GenericParam>> {
    let mut params = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter_declaration" {
            let mut name = String::new();
            let mut bounds = Vec::new();
            let mut c = child.walk();
            for ch in child.children(&mut c) {
                match ch.kind() {
                    "identifier" | "type_identifier" => name = text(ch, source).to_string(),
                    "type_constraint" => {
                        bounds.push(Type::Named(text(ch, source).to_string(), vec![]));
                    }
                    _ => {}
                }
            }
            params.push(GenericParam { name, bounds });
        }
    }
    Ok(params)
}

// =========================================================================
// Declarations (variables, forward declarations, etc.)
// =========================================================================

fn parse_declaration(node: Node, source: &str) -> Result<Vec<Item>> {
    let mut items = Vec::new();
    let mut base_ty = Type::Infer;
    let mut type_node_opt: Option<Node> = None;

    // Try the `type` field first, then fall back to type-like children.
    if let Some(type_node) = node.child_by_field_name("type") {
        base_ty = parse_type_node(type_node, source)?;
        type_node_opt = Some(type_node);
    } else {
        // tree-sitter-cpp may represent template types (e.g. vector<int>) as
        // a `template_type` child directly rather than as a `type` field.
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "template_type" | "qualified_identifier" | "type_identifier"
                | "primitive_type" | "sized_type_specifier" => {
                    base_ty = parse_type_node(child, source)?;
                    type_node_opt = Some(child);
                }
                _ => {}
            }
        }
    }

    // Build let stmts from init_declarator / declarator / bare identifier children.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "init_declarator" => {
                if let Ok((name, ty, init)) = parse_init_declarator(child, source, base_ty.clone(), type_node_opt) {
                    let let_stmt = Stmt::Let(LetStmt {
                        name: name.clone(),
                        ty: Some(ty.clone()),
                        init: init.map(Box::new),
                        mutable: true,
                    });
                    items.push(Item::Function(Function {
                        name,
                        generics: vec![],
                        params: vec![],
                        ret_ty: Some(ty),
                        body: Block {
                            stmts: vec![let_stmt],
                            expr: None,
                        },
                        is_unsafe: false,
                        is_virtual: false,
                        is_method: false,
                        self_param: None,
                    }));
                }
            }
            "function_declarator" => {
                let (name, params, _) = parse_function_declarator(child, source)?;
                items.push(Item::Function(Function {
                    name,
                    generics: vec![],
                    params,
                    ret_ty: Some(base_ty.clone()),
                    body: Block { stmts: vec![], expr: None },
                    is_unsafe: false,
                    is_virtual: false,
                    is_method: false,
                    self_param: None,
                }));
            }
            "declarator" | "reference_declarator" | "pointer_declarator" => {
                let name = extract_identifier_name(child, source);
                let mut ty = augment_type_with_declarator(base_ty.clone(), child, source).unwrap_or(base_ty.clone());
                if let Some(type_node) = type_node_opt {
                    ty = apply_const_from_type_node(ty.clone(), type_node, source);
                }
                let let_stmt = Stmt::Let(LetStmt {
                    name: name.clone(),
                    ty: Some(ty.clone()),
                    init: None,
                    mutable: true,
                });
                items.push(Item::Function(Function {
                    name,
                    generics: vec![],
                    params: vec![],
                    ret_ty: Some(ty),
                    body: Block {
                        stmts: vec![let_stmt],
                        expr: None,
                    },
                    is_unsafe: false,
                    is_virtual: false,
                    is_method: false,
                    self_param: None,
                }));
            }
            // Bare `identifier`: variable name without wrapper declarator node,
            // e.g. `vector<int> order;` where `order` is a plain identifier child.
            "identifier" if type_node_opt.is_some() => {
                let name = text(child, source).to_string();
                let ty = base_ty.clone();
                let let_stmt = Stmt::Let(LetStmt {
                    name: name.clone(),
                    ty: Some(ty.clone()),
                    init: None,
                    mutable: true,
                });
                items.push(Item::Function(Function {
                    name,
                    generics: vec![],
                    params: vec![],
                    ret_ty: Some(ty),
                    body: Block {
                        stmts: vec![let_stmt],
                        expr: None,
                    },
                    is_unsafe: false,
                    is_virtual: false,
                    is_method: false,
                    self_param: None,
                }));
            }
            _ => {}
        }
    }

    Ok(items)
}

fn parse_init_declarator(
    node: Node,
    source: &str,
    base_ty: Type,
    type_node_opt: Option<Node>,
) -> Result<(String, Type, Option<Expr>)> {
    let mut ty = base_ty;
    let mut init: Option<Expr> = None;

    let name = if let Some(decl_node) = node.child_by_field_name("declarator") {
        let n = extract_identifier_name(decl_node, source);
        ty = augment_type_with_declarator(ty.clone(), decl_node, source).unwrap_or(ty);
        if let Some(type_node) = type_node_opt {
            ty = apply_const_from_type_node(ty, type_node, source);
        }
        n
    } else {
        String::new()
    };

    if let Some(value_node) = node.child_by_field_name("value") {
        match value_node.kind() {
            "argument_list" => {
                init = Some(parse_call_expression_inner(&name, value_node, source)?);
            }
            _ => {
                if let Ok(expr) = parse_expression(value_node, source) {
                    init = Some(expr);
                }
            }
        }
    }

    // Fallback: scan children for initializer expressions
    if init.is_none() {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "declarator" | "=" => {}
                "argument_list" => {
                    init = Some(parse_call_expression_inner(&name, child, source)?);
                }
                _ => {
                    if let Ok(expr) = parse_expression(child, source) {
                        init = Some(expr);
                    }
                }
            }
        }
    }

    Ok((name, ty, init))
}

// =========================================================================
// Class / Struct
// =========================================================================

fn parse_class_specifier(node: Node, source: &str) -> Result<StructDef> {
    let mut name = String::new();
    let mut is_class = false;
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut generics = Vec::new();
    let mut base_classes = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class" => is_class = true,
            "struct" => is_class = false,
            "type_identifier" | "identifier" => name = text(child, source).to_string(),
            "field_declaration_list" => {
                let (f, m) = parse_field_declaration_list(child, source, &name)?;
                fields = f;
                methods = m;
            }
            "template_parameter_list" => {
                generics = parse_template_params(child, source)?;
            }
            "base_class_clause" => {
                base_classes = parse_base_class_clause(child, source);
            }
            "{" | "}" | ";" => {}
            _ => {}
        }
    }

    Ok(StructDef {
        name,
        generics,
        fields,
        is_class,
        methods,
        base_classes,
    })
}

/// Parse `base_class_clause`: extracts base class types with access specifiers.
fn parse_base_class_clause(node: Node, source: &str) -> Vec<BaseClass> {
    let mut bases = Vec::new();
    let mut pending_vis = Visibility::Public;
    let mut pending_virtual = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "access_specifier" => {
                pending_vis = match text(child, source) {
                    "public" => Visibility::Public,
                    "protected" => Visibility::Protected,
                    "private" => Visibility::Private,
                    _ => Visibility::Public,
                };
            }
            "virtual" => {
                pending_virtual = true;
            }
            "qualified_identifier" | "type_identifier" | "template_type" => {
                if let Ok(ty) = parse_type_node(child, source) {
                    bases.push(BaseClass {
                        ty,
                        visibility: pending_vis,
                        is_virtual: pending_virtual,
                    });
                }
                pending_vis = Visibility::Public;
                pending_virtual = false;
            }
            ":" | "," => {}
            _ => {}
        }
    }
    bases
}

fn parse_field_declaration_list(node: Node, source: &str, _struct_name: &str) -> Result<(Vec<FieldDef>, Vec<Function>)> {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let field_names: Vec<String> = {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|c| c.kind() == "field_declaration")
            .filter_map(|c| parse_field_declaration(c, source).ok())
            .map(|f| f.name)
            .collect()
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            if let Ok(f) = parse_field_declaration(child, source) {
                fields.push(f);
            }
        } else if child.kind() == "function_definition" || child.kind() == "virtual_function_definition" {
            if let Ok(mut m) = parse_function_definition(child, source) {
                m.is_method = true;
                m.self_param = Some(SelfParam::Ref);
                if child.kind() == "virtual_function_definition" {
                    m.is_virtual = true;
                }
                // In C++ member functions, `this` is implicit. We rewrite
                // `this` to `self` and unqualified field names to `self.field`.
                rewrite_this_in_block(&mut m.body, &field_names);
                methods.push(m);
            }
        }
    }
    Ok((fields, methods))
}

fn parse_field_declaration(node: Node, source: &str) -> Result<FieldDef> {
    let mut ty = Type::Infer;
    let mut name = String::new();
    let visibility = Visibility::Public;
    let mut type_node_opt: Option<Node> = None;

    if let Some(type_node) = node.child_by_field_name("type") {
        ty = parse_type_node(type_node, source)?;
        type_node_opt = Some(type_node);
    }

    if let Some(decl_node) = node.child_by_field_name("declarator") {
        name = extract_identifier_name(decl_node, source);
        ty = augment_type_with_declarator(ty.clone(), decl_node, source).unwrap_or(ty);
        if let Some(type_node) = type_node_opt {
            ty = apply_const_from_type_node(ty, type_node, source);
        }
    }

    Ok(FieldDef {
        name,
        ty,
        visibility,
    })
}

/// Rewrite `this` to `self` and bare field identifiers to `self.field`.
fn rewrite_this_in_block(block: &mut Block, field_names: &[String]) {
    for stmt in &mut block.stmts {
        rewrite_this_in_stmt(stmt, field_names);
    }
    if let Some(expr) = &mut block.expr {
        rewrite_this_in_expr(expr, field_names);
    }
}

fn rewrite_this_in_stmt(stmt: &mut Stmt, field_names: &[String]) {
    match stmt {
        Stmt::Let(l) => {
            if let Some(init) = &mut l.init {
                rewrite_this_in_expr(init, field_names);
            }
        }
        Stmt::Expr(e) | Stmt::Return(Some(e)) => {
            rewrite_this_in_expr(e, field_names);
        }
        Stmt::Assign(lhs, rhs) => {
            rewrite_this_in_expr(lhs, field_names);
            rewrite_this_in_expr(rhs, field_names);
        }
        Stmt::Block(b) => rewrite_this_in_block(b, field_names),
        Stmt::Return(None) => {}
    }
}

fn rewrite_this_in_expr(expr: &mut Expr, field_names: &[String]) {
    match expr {
        Expr::Ident(name) if name == "this" => {
            *name = "self".to_string();
        }
        Expr::Ident(name) if field_names.contains(name) => {
            *expr = Expr::Field(Box::new(Expr::Ident("self".to_string())), name.clone());
        }
        Expr::Binary(_, lhs, rhs) => {
            rewrite_this_in_expr(lhs, field_names);
            rewrite_this_in_expr(rhs, field_names);
        }
        Expr::Unary(_, inner) => {
            rewrite_this_in_expr(inner, field_names);
        }
        Expr::Call(callee, args) => {
            rewrite_this_in_expr(callee, field_names);
            for arg in args {
                rewrite_this_in_expr(arg, field_names);
            }
        }
        Expr::MethodCall(receiver, _, args) => {
            rewrite_this_in_expr(receiver, field_names);
            for arg in args {
                rewrite_this_in_expr(arg, field_names);
            }
        }
        Expr::Field(obj, _) => {
            rewrite_this_in_expr(obj, field_names);
        }
        Expr::Index(arr, idx) => {
            rewrite_this_in_expr(arr, field_names);
            rewrite_this_in_expr(idx, field_names);
        }
        Expr::If(cond, then_b, else_b) => {
            rewrite_this_in_expr(cond, field_names);
            rewrite_this_in_block(then_b, field_names);
            if let Some(b) = else_b {
                rewrite_this_in_block(b, field_names);
            }
        }
        Expr::Match(scrutinee, arms) => {
            rewrite_this_in_expr(scrutinee, field_names);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    rewrite_this_in_expr(guard, field_names);
                }
                rewrite_this_in_expr(&mut arm.body, field_names);
            }
        }
        Expr::While(cond, body) => {
            rewrite_this_in_expr(cond, field_names);
            rewrite_this_in_block(body, field_names);
        }
        Expr::For(head, body) => {
            rewrite_this_in_expr(&mut head.expr, field_names);
            rewrite_this_in_block(body, field_names);
        }
        Expr::Block(b) => rewrite_this_in_block(b, field_names),
        Expr::StructInit(_, fields) => {
            for (_, fexpr) in fields {
                rewrite_this_in_expr(fexpr, field_names);
            }
        }
        Expr::ArrayInit(elems) | Expr::Tuple(elems) => {
            for elem in elems {
                rewrite_this_in_expr(elem, field_names);
            }
        }
        _ => {}
    }
}

// =========================================================================
// Enum
// =========================================================================

fn parse_enum_specifier(node: Node, source: &str) -> Result<EnumDef> {
    let mut name = String::new();
    let mut variants = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_identifier" | "identifier" => name = text(child, source).to_string(),
            "enumerator_list" => {
                let mut c2 = child.walk();
                for ch in child.children(&mut c2) {
                    if ch.kind() == "enumerator" {
                        let mut vname = String::new();
                        let mut c3 = ch.walk();
                        for ch2 in ch.children(&mut c3) {
                            if ch2.kind() == "identifier" {
                                vname = text(ch2, source).to_string();
                            }
                        }
                        variants.push(VariantDef {
                            name: vname,
                            fields: VariantFields::Unit,
                        });
                    }
                }
            }
            "{" | "}" | ";" => {}
            _ => {}
        }
    }

    Ok(EnumDef {
        name,
        generics: vec![],
        variants,
    })
}

/// Parse a single statement node into one or more IR Stmts.
fn parse_statement_node(node: Node, source: &str) -> Result<Vec<Stmt>> {
    match node.kind() {
        "compound_statement" => {
            let block = parse_compound_statement(node, source)?;
            Ok(vec![Stmt::Block(block)])
        }
        "return_statement" => Ok(vec![parse_return_statement(node, source)?]),
        "if_statement" => Ok(vec![parse_if_statement(node, source)?]),
        "for_statement" => Ok(vec![parse_for_statement(node, source)?]),
        "range_for_statement" | "for_range_loop" => Ok(vec![parse_range_for_statement(node, source)?]),
        "while_statement" => Ok(vec![parse_while_statement(node, source)?]),
        "expression_statement" => Ok(vec![parse_expression_statement(node, source)?]),
        "declaration" => {
            let items = parse_declaration(node, source)?;
            let mut stmts = Vec::new();
            for item in items {
                match item {
                    Item::Function(f) if f.body.stmts.len() == 1 => {
                        if let Stmt::Let(l) = &f.body.stmts[0] {
                            stmts.push(Stmt::Let(l.clone()));
                        }
                    }
                    _ => {
                        stmts.push(Stmt::Expr(Box::new(Expr::TODO(format!("{:?}", item)))));
                    }
                }
            }
            Ok(stmts)
        }
        "break_statement" => Ok(vec![Stmt::Expr(Box::new(Expr::Break))]),
        "continue_statement" => Ok(vec![Stmt::Expr(Box::new(Expr::Continue))]),
        "jump_statement" => {
            let txt = text(node, source).trim();
            if txt.starts_with("break") {
                Ok(vec![Stmt::Expr(Box::new(Expr::Break))])
            } else if txt.starts_with("continue") {
                Ok(vec![Stmt::Expr(Box::new(Expr::Continue))])
            } else if txt.starts_with("return") {
                Ok(vec![parse_return_statement(node, source)?])
            } else {
                Ok(vec![Stmt::Expr(Box::new(Expr::TODO(format!("jump: {}", txt))))])
            }
        }
        "assignment_expression" | "compound_assignment_expr" | "update_expression"
        | "call_expression" | "binary_expression" | "unary_expression"
        | "subscript_expression" | "field_expression" => {
            if let Ok(expr) = parse_expression(node, source) {
                Ok(vec![Stmt::Expr(Box::new(expr))])
            } else {
                Ok(vec![])
            }
        }
        _ => Ok(vec![]),
    }
}

// =========================================================================
// Statements
// =========================================================================

fn parse_compound_statement(node: Node, source: &str) -> Result<Block> {
    let mut stmts = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "compound_statement" => {
                stmts.push(Stmt::Block(parse_compound_statement(child, source)?));
            }
            "return_statement" => {
                stmts.push(parse_return_statement(child, source)?);
            }
            "if_statement" => {
                stmts.push(parse_if_statement(child, source)?);
            }
            "for_statement" => {
                stmts.push(parse_for_statement(child, source)?);
            }
            "range_for_statement" | "for_range_loop" => {
                stmts.push(parse_range_for_statement(child, source)?);
            }
            "while_statement" => {
                stmts.push(parse_while_statement(child, source)?);
            }
            "declaration" => {
                // local variable declaration
                let items = parse_declaration(child, source)?;
                for item in items {
                    match item {
                        Item::Function(f) if f.body.stmts.len() == 1 => {
                            if let Stmt::Let(l) = &f.body.stmts[0] {
                                stmts.push(Stmt::Let(l.clone()));
                            }
                        }
                        _ => {
                            stmts.push(Stmt::Expr(Box::new(Expr::TODO(format!("{:?}", item)))));
                        }
                    }
                }
            }
            "expression_statement" => {
                stmts.push(parse_expression_statement(child, source)?);
            }
            "assignment_expression" | "compound_assignment_expr" | "update_expression" | "call_expression" | "binary_expression" | "unary_expression" | "subscript_expression" | "field_expression" => {
                if let Ok(expr) = parse_expression(child, source) {
                    stmts.push(Stmt::Expr(Box::new(expr)));
                }
            }
            "break_statement" => stmts.push(Stmt::Expr(Box::new(Expr::Break))),
            "continue_statement" => stmts.push(Stmt::Expr(Box::new(Expr::Continue))),
            "jump_statement" => {
                let txt = text(node, source).trim();
                if txt.starts_with("break") {
                    stmts.push(Stmt::Expr(Box::new(Expr::Break)));
                } else if txt.starts_with("continue") {
                    stmts.push(Stmt::Expr(Box::new(Expr::Continue)));
                } else if txt.starts_with("return") {
                    stmts.push(parse_return_statement(node, source)?);
                } else {
                    stmts.push(Stmt::Expr(Box::new(Expr::TODO(format!("jump: {}", txt)))));
                }
            }
            "comment" | ";" | "{" | "}" => {}
            _ => {
                stmts.push(Stmt::Expr(Box::new(Expr::TODO(format!(
                    "stmt: {}",
                    child.kind()
                )))));
            }
        }
    }
    Ok(Block { stmts, expr: None })
}

fn parse_return_statement(node: Node, source: &str) -> Result<Stmt> {
    let mut expr: Option<Box<Expr>> = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "return" && child.kind() != ";" {
            if let Ok(e) = parse_expression(child, source) {
                expr = Some(Box::new(e));
            }
        }
    }
    Ok(Stmt::Return(expr))
}

fn parse_if_statement(node: Node, source: &str) -> Result<Stmt> {
    let mut cond = Expr::Literal(Literal::Bool(true));
    let mut then_branch = Block {
        stmts: vec![],
        expr: None,
    };
    let mut else_branch: Option<Block> = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if" | "else" => {}
            "parenthesized_expression" | "condition_clause" => {
                if let Ok(e) = parse_expression(child, source) {
                    cond = e;
                }
            }
            "compound_statement" => {
                if then_branch.stmts.is_empty() {
                    then_branch = parse_compound_statement(child, source)?;
                } else {
                    else_branch = Some(parse_compound_statement(child, source)?);
                }
            }
            "if_statement" => {
                // else if
                else_branch = Some(Block {
                    stmts: vec![parse_if_statement(child, source)?],
                    expr: None,
                });
            }
            _ => {
                if then_branch.stmts.is_empty() {
                    let stmts = parse_statement_node(child, source)?;
                    if !stmts.is_empty() {
                        then_branch.stmts = stmts;
                    }
                } else {
                    let stmts = parse_statement_node(child, source)?;
                    if !stmts.is_empty() {
                        else_branch = Some(Block { stmts, expr: None });
                    }
                }
            }
        }
    }

    Ok(Stmt::Expr(Box::new(Expr::If(
        Box::new(cond),
        then_branch,
        else_branch,
    ))))
}

fn parse_for_statement(node: Node, source: &str) -> Result<Stmt> {
    let mut inits: Vec<Stmt> = Vec::new();
    let mut cond: Option<Expr> = None;
    let mut update: Option<Expr> = None;
    let mut body = Block {
        stmts: vec![],
        expr: None,
    };

    // Use child_by_field_name for reliable extraction
    if let Some(init_node) = node.child_by_field_name("initializer") {
        match init_node.kind() {
            "declaration" => {
                let items = parse_declaration(init_node, source)?;
                for item in items {
                    if let Item::Function(f) = item {
                        if let Some(Stmt::Let(l)) = f.body.stmts.first() {
                            inits.push(Stmt::Let(l.clone()));
                        }
                    }
                }
            }
            "expression_statement" => {
                inits.push(parse_expression_statement(init_node, source)?);
            }
            _ => {
                if let Ok(e) = parse_expression(init_node, source) {
                    inits.push(Stmt::Expr(Box::new(e)));
                }
            }
        }
    }

    if let Some(cond_node) = node.child_by_field_name("condition") {
        if let Ok(e) = parse_expression(cond_node, source) {
            cond = Some(e);
        }
    }

    if let Some(update_node) = node.child_by_field_name("update") {
        if let Ok(e) = parse_expression(update_node, source) {
            update = Some(e);
        }
    }

    if let Some(body_node) = node.child_by_field_name("body") {
        body = parse_compound_statement(body_node, source)?;
    }

    // Desugar to while loop
    let mut stmts = Vec::new();
    stmts.extend(inits);

    let while_body = Block {
        stmts: {
            let mut b = body.stmts.clone();
            if let Some(u) = update {
                b.push(Stmt::Expr(Box::new(u)));
            }
            b
        },
        expr: body.expr.clone(),
    };

    stmts.push(Stmt::Expr(Box::new(Expr::While(
        Box::new(cond.unwrap_or(Expr::Literal(Literal::Bool(true)))),
        while_body,
    ))));

    Ok(Stmt::Block(Block { stmts, expr: None }))
}

fn parse_range_for_statement(node: Node, source: &str) -> Result<Stmt> {
    let mut pat = Pattern::Wild;
    let mut expr = Expr::Literal(Literal::Bool(true));
    let mut body = Block {
        stmts: vec![],
        expr: None,
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "for" | ";" | "(" | ")" | ":" => {}
            "declaration" => {
                // Directly extract the variable name from declarators
                let mut c = child.walk();
                for ch in child.children(&mut c) {
                    match ch.kind() {
                        "init_declarator" | "declarator" | "reference_declarator" | "pointer_declarator" => {
                            let name = extract_identifier_name(ch, source);
                            if !name.is_empty() {
                                pat = Pattern::Ident(name);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "_declaration_specifiers" | "primitive_type" | "type_identifier" | "sized_type_specifier" => {}
            "reference_declarator" | "pointer_declarator" | "declarator" => {
                let name = extract_identifier_name(child, source);
                if !name.is_empty() {
                    pat = Pattern::Ident(name);
                }
            }
            "compound_statement" => {
                body = parse_compound_statement(child, source)?;
            }
            _ => {
                if let Ok(e) = parse_expression(child, source) {
                    expr = e;
                }
            }
        }
    }

    Ok(Stmt::Expr(Box::new(Expr::For(
        Box::new(ForHead {
            pat,
            expr: Box::new(expr),
        }),
        body,
    ))))
}

fn parse_while_statement(node: Node, source: &str) -> Result<Stmt> {
    let mut cond = Expr::Literal(Literal::Bool(true));
    let mut body = Block {
        stmts: vec![],
        expr: None,
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "while" => {}
            "parenthesized_expression" | "condition_clause" => {
                if let Ok(e) = parse_expression(child, source) {
                    cond = e;
                }
            }
            "compound_statement" => {
                body = parse_compound_statement(child, source)?;
            }
            _ => {}
        }
    }

    Ok(Stmt::Expr(Box::new(Expr::While(Box::new(cond), body))))
}

fn parse_expression_statement(node: Node, source: &str) -> Result<Stmt> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != ";" {
            if let Ok(expr) = parse_expression(child, source) {
                return Ok(Stmt::Expr(Box::new(expr)));
            }
            // Fallback: search one level deeper
            let mut c2 = child.walk();
            for ch2 in child.children(&mut c2) {
                if let Ok(expr) = parse_expression(ch2, source) {
                    return Ok(Stmt::Expr(Box::new(expr)));
                }
            }
        }
    }
    Ok(Stmt::Expr(Box::new(Expr::TODO(
        "empty expression statement".to_string(),
    ))))
}

// =========================================================================
// Expressions
// =========================================================================

fn parse_expression(node: Node, source: &str) -> Result<Expr> {
    match node.kind() {
        "number_literal" => Ok(Expr::Literal(Literal::Int(text(node, source).to_string()))),
        "string_literal" | "raw_string_literal" => {
            let s = text(node, source);
            // Remove surrounding quotes for MVP
            let s = s.trim_start_matches('"').trim_end_matches('"');
            Ok(Expr::Literal(Literal::String(s.to_string())))
        }
        "char_literal" => {
            let s = text(node, source);
            let c = s.chars().nth(1).unwrap_or('\0');
            Ok(Expr::Literal(Literal::Char(c)))
        }
        "true" => Ok(Expr::Literal(Literal::Bool(true))),
        "false" => Ok(Expr::Literal(Literal::Bool(false))),
        "identifier" => Ok(Expr::Ident(text(node, source).to_string())),
        "qualified_identifier" => Ok(Expr::Ident(text(node, source).to_string())),
        "this" => Ok(Expr::Ident("self".to_string())),
        "null" | "nullptr" => Ok(Expr::Ident("None".to_string())),
        "binary_expression" => parse_binary_expression(node, source),
        "unary_expression" => parse_unary_expression(node, source),
        "call_expression" => parse_call_expression(node, source),
        "field_expression" => parse_field_expression(node, source),
        "subscript_expression" => parse_subscript_expression(node, source),
        "parenthesized_expression" => {
            let inner = node.named_child(0).ok_or_else(|| anyhow::anyhow!("empty paren expr"))?;
            parse_expression(inner, source)
        }
        "condition_clause" => {
            // C++17 if/while condition: (declaration; expr) or (expr)
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != ";" {
                    if let Ok(e) = parse_expression(child, source) {
                        return Ok(e);
                    }
                }
            }
            // Fallback: search one level deeper for nested expressions
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != ";" {
                    let mut c2 = child.walk();
                    for ch2 in child.children(&mut c2) {
                        if let Ok(e) = parse_expression(ch2, source) {
                            return Ok(e);
                        }
                    }
                }
            }
            Ok(Expr::TODO("condition_clause".to_string()))
        }
        "subscript_argument_list" => {
            if let Some(inner) = node.named_child(0) {
                parse_expression(inner, source)
            } else {
                Ok(Expr::TODO("empty subscript arg list".to_string()))
            }
        }
        "update_expression" => parse_update_expression(node, source),
        "assignment_expression" => parse_assignment_expression(node, source),
        "compound_assignment_expr" => parse_compound_assignment_expression(node, source),
        "conditional_expression" => parse_conditional_expression(node, source),
        "cast_expression" => parse_cast_expression(node, source),
        "new_expression" => parse_new_expression(node, source),
        "delete_expression" => parse_delete_expression(node, source),
        "sizeof_expression" => Ok(Expr::TODO("sizeof".to_string())),
        "lambda_expression" => parse_lambda_expression(node, source),
        "template_type" | "template_function" => {
            let name = extract_identifier_name(node, source);
            Ok(Expr::Ident(name))
        }
        "initializer_list" => parse_initializer_list(node, source),
        "compound_literal_expression" => parse_initializer_list(node, source),
        "type" => Ok(Expr::TODO("type as expression".to_string())),
        _ => Ok(Expr::TODO(format!("expr: {}", node.kind()))),
    }
}

fn parse_binary_expression(node: Node, source: &str) -> Result<Expr> {
    let op_node = node.child_by_field_name("operator").ok_or_else(|| anyhow::anyhow!("no op"))?;
    let op = parse_bin_op(text(op_node, source))?;
    let left = node
        .child_by_field_name("left")
        .ok_or_else(|| anyhow::anyhow!("no left"))?;
    let right = node
        .child_by_field_name("right")
        .ok_or_else(|| anyhow::anyhow!("no right"))?;
    Ok(Expr::Binary(
        op,
        Box::new(parse_expression(left, source)?),
        Box::new(parse_expression(right, source)?),
    ))
}

fn parse_bin_op(op: &str) -> Result<BinOp> {
    match op {
        "+" => Ok(BinOp::Add),
        "-" => Ok(BinOp::Sub),
        "*" => Ok(BinOp::Mul),
        "/" => Ok(BinOp::Div),
        "%" => Ok(BinOp::Rem),
        "&&" => Ok(BinOp::And),
        "||" => Ok(BinOp::Or),
        "==" => Ok(BinOp::Eq),
        "!=" => Ok(BinOp::Ne),
        "<" => Ok(BinOp::Lt),
        "<=" => Ok(BinOp::Le),
        ">" => Ok(BinOp::Gt),
        ">=" => Ok(BinOp::Ge),
        "&" => Ok(BinOp::BitAnd),
        "|" => Ok(BinOp::BitOr),
        "^" => Ok(BinOp::BitXor),
        "<<" => Ok(BinOp::Shl),
        ">>" => Ok(BinOp::Shr),
        "=" => Ok(BinOp::Assign),
        "+=" => Ok(BinOp::AddAssign),
        "-=" => Ok(BinOp::SubAssign),
        "*=" => Ok(BinOp::MulAssign),
        "/=" => Ok(BinOp::DivAssign),
        "%=" => Ok(BinOp::RemAssign),
        _ => bail!("unknown binary operator: {}", op),
    }
}

fn parse_unary_expression(node: Node, source: &str) -> Result<Expr> {
    let op_node = node.child_by_field_name("operator").ok_or_else(|| anyhow::anyhow!("no op"))?;
    let op = match text(op_node, source) {
        "-" => UnOp::Neg,
        "!" => UnOp::Not,
        "*" => UnOp::Deref,
        "&" => UnOp::Ref,
        "~" => {
            return Ok(Expr::TODO("bitwise not".to_string()));
        }
        _ => bail!("unknown unary operator"),
    };
    let arg = node
        .child_by_field_name("argument")
        .ok_or_else(|| anyhow::anyhow!("no arg"))?;
    Ok(Expr::Unary(op, Box::new(parse_expression(arg, source)?)))
}

fn parse_call_expression(node: Node, source: &str) -> Result<Expr> {
    let func = node.child_by_field_name("function");
    let args = node.child_by_field_name("arguments");

    if let (Some(f), Some(a)) = (func, args) {
        let callee = parse_expression(f, source)?;
        let mut arg_exprs = Vec::new();
        let mut cursor = a.walk();
        for child in a.children(&mut cursor) {
            if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                if let Ok(e) = parse_expression(child, source) {
                    arg_exprs.push(e);
                }
            }
        }
        Ok(Expr::Call(Box::new(callee), arg_exprs))
    } else {
        Ok(Expr::TODO("call".to_string()))
    }
}

fn parse_call_expression_inner(name: &str, args_node: Node, source: &str) -> Result<Expr> {
    let mut arg_exprs = Vec::new();
    let mut cursor = args_node.walk();
    for child in args_node.children(&mut cursor) {
        if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
            if let Ok(e) = parse_expression(child, source) {
                arg_exprs.push(e);
            }
        }
    }
    Ok(Expr::Call(
        Box::new(Expr::Ident(name.to_string())),
        arg_exprs,
    ))
}

fn parse_field_expression(node: Node, source: &str) -> Result<Expr> {
    let arg = node.child_by_field_name("argument");
    let field = node.child_by_field_name("field");

    if let (Some(a), Some(f)) = (arg, field) {
        let obj = parse_expression(a, source)?;
        let fname = text(f, source).to_string();
        Ok(Expr::Field(Box::new(obj), fname))
    } else {
        Ok(Expr::TODO("field".to_string()))
    }
}

fn parse_subscript_expression(node: Node, source: &str) -> Result<Expr> {
    let mut arr = node.child_by_field_name("argument");
    let mut idx = node.child_by_field_name("index");

    if arr.is_none() || idx.is_none() {
        let mut cursor = node.walk();
        let mut children = Vec::new();
        for child in node.children(&mut cursor) {
            if child.kind() != "[" && child.kind() != "]" {
                children.push(child);
            }
        }
        if children.len() >= 2 {
            arr = Some(children[0]);
            idx = Some(children[1]);
        }
    }

    if let (Some(a), Some(i)) = (arr, idx) {
        Ok(Expr::Index(
            Box::new(parse_expression(a, source)?),
            Box::new(parse_expression(i, source)?),
        ))
    } else {
        Ok(Expr::TODO("subscript".to_string()))
    }
}

fn parse_update_expression(node: Node, source: &str) -> Result<Expr> {
    let op_node = node.child_by_field_name("operator").ok_or_else(|| anyhow::anyhow!("no op"))?;
    let op = text(op_node, source);
    let arg = node
        .child_by_field_name("argument")
        .ok_or_else(|| anyhow::anyhow!("no arg"))?;
    let expr = parse_expression(arg, source)?;
    if op == "++" {
        Ok(Expr::Binary(
            BinOp::AddAssign,
            Box::new(expr),
            Box::new(Expr::Literal(Literal::Int("1".to_string()))),
        ))
    } else if op == "--" {
        Ok(Expr::Binary(
            BinOp::SubAssign,
            Box::new(expr),
            Box::new(Expr::Literal(Literal::Int("1".to_string()))),
        ))
    } else {
        bail!("unknown update op")
    }
}

fn parse_assignment_expression(node: Node, source: &str) -> Result<Expr> {
    let left = node
        .child_by_field_name("left")
        .ok_or_else(|| anyhow::anyhow!("no left"))?;
    let right = node
        .child_by_field_name("right")
        .ok_or_else(|| anyhow::anyhow!("no right"))?;
    // Detect compound assignment (*=, +=, -=, etc.) vs plain =
    let op = if let Some(op_node) = node.child_by_field_name("operator") {
        parse_bin_op(text(op_node, source)).unwrap_or(BinOp::Assign)
    } else {
        BinOp::Assign
    };
    Ok(Expr::Binary(
        op,
        Box::new(parse_expression(left, source)?),
        Box::new(parse_expression(right, source)?),
    ))
}

fn parse_compound_assignment_expression(node: Node, source: &str) -> Result<Expr> {
    let op_node = node.child_by_field_name("operator").ok_or_else(|| anyhow::anyhow!("no op"))?;
    let op = parse_bin_op(text(op_node, source))?;
    let left = node
        .child_by_field_name("left")
        .ok_or_else(|| anyhow::anyhow!("no left"))?;
    let right = node
        .child_by_field_name("right")
        .ok_or_else(|| anyhow::anyhow!("no right"))?;
    Ok(Expr::Binary(
        op,
        Box::new(parse_expression(left, source)?),
        Box::new(parse_expression(right, source)?),
    ))
}

fn parse_conditional_expression(node: Node, source: &str) -> Result<Expr> {
    let mut cond = Expr::Literal(Literal::Bool(true));
    let mut then_expr = Expr::Literal(Literal::Bool(true));
    let mut else_expr = Expr::Literal(Literal::Bool(true));

    let mut cursor = node.walk();
    let mut state = 0;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "?" => state = 1,
            ":" => state = 2,
            _ => {
                if let Ok(e) = parse_expression(child, source) {
                    match state {
                        0 => cond = e,
                        1 => then_expr = e,
                        2 => else_expr = e,
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(Expr::If(
        Box::new(cond),
        Block {
            stmts: vec![],
            expr: Some(Box::new(then_expr)),
        },
        Some(Block {
            stmts: vec![],
            expr: Some(Box::new(else_expr)),
        }),
    ))
}

fn parse_cast_expression(node: Node, source: &str) -> Result<Expr> {
    let mut _type_node: Option<Node> = None;
    let mut value_node: Option<Node> = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "type" => _type_node = Some(child),
            "parenthesized_expression" | "expression" | "identifier" | "number_literal" => {
                value_node = Some(child);
            }
            "(" | ")" => {}
            _ => {
                if value_node.is_none() {
                    value_node = Some(child);
                }
            }
        }
    }

    if let Some(vn) = value_node {
        parse_expression(vn, source)
    } else {
        Ok(Expr::TODO("cast without value".to_string()))
    }
}

fn parse_new_expression(node: Node, source: &str) -> Result<Expr> {
    let mut type_node: Option<Node> = None;
    let mut args: Option<Node> = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "new" => {}
            "type" => type_node = Some(child),
            "argument_list" => args = Some(child),
            "parenthesized_expression" => args = Some(child),
            _ => {}
        }
    }

    let _ty = type_node.map(|n| parse_type_node(n, source).unwrap_or(Type::Infer));
    let mut arg_exprs = Vec::new();
    if let Some(a) = args {
        let mut c = a.walk();
        for ch in a.children(&mut c) {
            if ch.kind() != "(" && ch.kind() != ")" && ch.kind() != "," {
                if let Ok(e) = parse_expression(ch, source) {
                    arg_exprs.push(e);
                }
            }
        }
    }

    Ok(Expr::Call(
        Box::new(Expr::Ident("Box::new".to_string())),
        arg_exprs,
    ))
}

fn parse_delete_expression(_node: Node, _source: &str) -> Result<Expr> {
    // In Rust, Drop handles this. We'll emit a TODO comment via expression.
    Ok(Expr::TODO("delete -> manual Drop review needed".to_string()))
}

fn parse_lambda_expression(node: Node, source: &str) -> Result<Expr> {
    let mut params = Vec::new();
    let mut ret_ty: Option<Type> = None;
    let mut body = Block { stmts: vec![], expr: None };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "lambda_capture_specifier" => {
                // MVP: ignore captures, default to Rust closure semantics
            }
            "lambda_declarator" => {
                let mut c = child.walk();
                for ch in child.children(&mut c) {
                    match ch.kind() {
                        "parameter_list" => {
                            params = parse_parameter_list(ch, source)?;
                        }
                        "trailing_return_type" => {
                            // e.g. "-> vector<int>"
                            let mut c2 = ch.walk();
                            for ch2 in ch.children(&mut c2) {
                                if ch2.kind() == "type" {
                                    if let Ok(t) = parse_type_node(ch2, source) {
                                        ret_ty = Some(t);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            "compound_statement" => {
                body = parse_compound_statement(child, source)?;
            }
            _ => {}
        }
    }

    Ok(Expr::Closure(params, ret_ty, body))
}

fn parse_initializer_list(node: Node, source: &str) -> Result<Expr> {
    let mut elems = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "{" && child.kind() != "}" && child.kind() != "," {
            if let Ok(e) = parse_expression(child, source) {
                elems.push(e);
            }
        }
    }
    // Detect vector<int>{} pattern: [Ident("vector"), ArrayInit([])]
    if elems.len() == 2
        && matches!(&elems[0], Expr::Ident(name) if name == "vector" || name == "std::vector")
        && matches!(&elems[1], Expr::ArrayInit(v) if v.is_empty())
    {
        Ok(Expr::Call(Box::new(Expr::Ident("Vec::new".to_string())), vec![]))
    } else {
        Ok(Expr::ArrayInit(elems))
    }
}

// =========================================================================
// Types
// =========================================================================

fn parse_type_node(node: Node, source: &str) -> Result<Type> {
    let txt = text(node, source).trim();
    if txt.is_empty() {
        return Ok(Type::Infer);
    }

    // Try naive text-based parsing first for common std types
    if let Ok(t) = cpp_type_to_ir(txt) {
        return Ok(t);
    }

    // Handle `template_type` nodes (e.g. `queue<int>`, `std::vector<int>`).
    if node.kind() == "template_type" {
        let mut base_name = String::new();
        let mut type_args = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" | "qualified_identifier" | "primitive_type" => {
                    base_name = text(child, source).to_string();
                }
                "template_argument_list" => {
                    let mut c2 = child.walk();
                    for arg in child.children(&mut c2) {
                        match arg.kind() {
                            "type_descriptor" | "type" | "template_type"
                            | "primitive_type" | "type_identifier" | "qualified_identifier" => {
                                if let Ok(t) = parse_type_node(arg, source) {
                                    type_args.push(t);
                                }
                            }
                            "<" | ">" | "," => {}
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        if !base_name.is_empty() {
            return Ok(Type::Named(base_name, type_args));
        }
    }

    // Otherwise walk children
    let mut primitive = String::new();
    let mut _is_const = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "primitive_type" | "type_identifier" | "sized_type_specifier" | "type_parameter" => {
                if !primitive.is_empty() {
                    primitive.push(' ');
                }
                primitive.push_str(text(child, source));
            }
            "type_qualifier" => {
                if text(child, source) == "const" {
                    _is_const = true;
                }
            }
            "qualified_identifier" => {
                primitive = text(child, source).to_string();
            }
            "template_argument_list" => {
                // handled above for template_type; fall through for nested
            }
            "_" => {}
            _ => {}
        }
    }

    if !primitive.is_empty() {
        if let Ok(t) = cpp_type_to_ir(&primitive) {
            return Ok(t);
        }
        return Ok(Type::Named(primitive, vec![]));
    }

    Ok(Type::Named(txt.to_string(), vec![]))
}

/// If the original type node had a top-level `const` qualifier, apply it
/// to reference/pointer types (which is where C++ puts the mutability).
fn apply_const_from_type_node(ty: Type, type_node: Node, source: &str) -> Type {
    let mut is_const = false;
    let mut cursor = type_node.walk();
    for child in type_node.children(&mut cursor) {
        if child.kind() == "type_qualifier" && text(child, source) == "const" {
            is_const = true;
            break;
        }
    }
    if is_const {
        match ty {
            Type::Ref(inner, _) => Type::Ref(inner, Mutability::Not),
            Type::Ptr(inner, _) => Type::Ptr(inner, Mutability::Not),
            other => other,
        }
    } else {
        ty
    }
}

fn augment_type_with_declarator(base: Type, node: Node, source: &str) -> Result<Type> {
    match node.kind() {
        "pointer_declarator" | "abstract_pointer_declarator" => {
            let mut is_const = false;
            let mut inner = base.clone();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "type_qualifier" => {
                        if text(child, source) == "const" {
                            is_const = true;
                        }
                    }
                    "identifier" => {}
                    _ => {
                        if let Ok(t) = augment_type_with_declarator(base.clone(), child, source) {
                            inner = t;
                        }
                    }
                }
            }
            Ok(Type::Ptr(
                Box::new(inner),
                if is_const {
                    Mutability::Not
                } else {
                    Mutability::Mut
                },
            ))
        }
        "reference_declarator" => {
            let mut is_const = false;
            let mut inner = base.clone();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "type_qualifier" => {
                        if text(child, source) == "const" {
                            is_const = true;
                        }
                    }
                    "identifier" => {}
                    _ => {
                        if let Ok(t) = augment_type_with_declarator(base.clone(), child, source) {
                            inner = t;
                        }
                    }
                }
            }
            Ok(Type::Ref(
                Box::new(inner),
                if is_const {
                    Mutability::Not
                } else {
                    Mutability::Mut
                },
            ))
        }
        _ => Ok(base),
    }
}

/// Split "foo, bar<baz>, qux" by top-level commas (respecting nested <>).
fn split_type_args(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(s[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = s[start..].trim().to_string();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

/// Convert a C++ type string into an IR `Type`.
pub fn cpp_type_to_ir(ty: &str) -> Result<Type> {
    let ty = ty.trim();
    if ty.is_empty() {
        return Ok(Type::Infer);
    }
    if ty == "void" {
        Ok(Type::Unit)
    } else if ty == "auto" {
        Ok(Type::Infer)
    } else if ty == "int" || ty == "short" || ty == "char" {
        Ok(Type::Named(ty.to_string(), vec![]))
    } else if ty == "long" || ty == "long long" {
        Ok(Type::Named("long".to_string(), vec![]))
    } else if ty == "bool" {
        Ok(Type::Named("bool".to_string(), vec![]))
    } else if ty == "std::queue" || ty == "queue" {
        Ok(Type::Named("Queue".to_string(), vec![]))
    } else if ty == "std::string" || ty == "string" {
        Ok(Type::Named("String".to_string(), vec![]))
    } else if ty.starts_with("std::vector<") && ty.ends_with(">") {
        let inner = &ty[12..ty.len() - 1];
        Ok(Type::Named("Vec".to_string(), vec![cpp_type_to_ir(inner)?]))
    } else if ty.starts_with("vector<") && ty.ends_with(">") {
        let inner = &ty[7..ty.len() - 1];
        Ok(Type::Named("Vec".to_string(), vec![cpp_type_to_ir(inner)?]))
    } else if ty.starts_with("std::unique_ptr<") && ty.ends_with(">") {
        let inner = &ty[16..ty.len() - 1];
        Ok(Type::Named("Box".to_string(), vec![cpp_type_to_ir(inner)?]))
    } else if ty.starts_with("std::shared_ptr<") && ty.ends_with(">") {
        let inner = &ty[16..ty.len() - 1];
        Ok(Type::Named("Rc".to_string(), vec![cpp_type_to_ir(inner)?]))
    } else if ty.starts_with("const ") && ty.ends_with("&") {
        let inner = ty[6..ty.len() - 1].trim();
        Ok(Type::Ref(Box::new(cpp_type_to_ir(inner)?), Mutability::Not))
    } else if ty.ends_with("&") && !ty.ends_with("&&") {
        let inner = &ty[..ty.len() - 1];
        Ok(Type::Ref(Box::new(cpp_type_to_ir(inner)?), Mutability::Mut))
    } else if ty.ends_with("*") {
        let inner = &ty[..ty.len() - 1];
        Ok(Type::Ptr(Box::new(cpp_type_to_ir(inner)?), Mutability::Mut))
    } else if let Some(open) = ty.find('<') {
        // Generic: `base<args...>`
        if ty.ends_with('>') {
            let base = ty[..open].trim();
            let args_str = &ty[open + 1..ty.len() - 1];
            let args: Vec<Type> = split_type_args(args_str)
                .iter()
                .map(|a| cpp_type_to_ir(a).unwrap_or(Type::Infer))
                .collect();
            // Map known types
            let ir_base = match base {
                "std::queue" | "queue" => "Queue",
                "std::vector" | "vector" => "Vec",
                "std::unique_ptr" | "unique_ptr" => "Box",
                "std::shared_ptr" | "shared_ptr" => "Rc",
                "std::string" | "string" => return Ok(Type::Named("String".to_string(), vec![])),
                "std::optional" | "optional" => "Option",
                other => other,
            };
            Ok(Type::Named(ir_base.to_string(), args))
        } else {
            Ok(Type::Named(ty.to_string(), vec![]))
        }
    } else {
        Ok(Type::Named(ty.to_string(), vec![]))
    }
}
