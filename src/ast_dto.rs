use cp_ast_core::constraint::{
    ArithOp, CharSetSpec, Constraint, ConstraintId, ConstraintSet, DistinctUnit, ExpectedType,
    Expression, PropertyTag, RelationOp, RenderHintKind, Separator, SortOrder,
};
use cp_ast_core::operation::AstEngine;
use cp_ast_core::structure::{
    Ident, Literal, NodeId, NodeKind, NodeKindHint, Reference, StructureAst, StructureNode,
};
use serde::Deserialize;
use thiserror::Error;

const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum AstDtoError {
    #[error("invalid AST JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported schema version: {0}")]
    UnsupportedVersion(u32),
    #[error("invalid id: {0}")]
    InvalidId(String),
    #[error("arena slot id mismatch: expected {expected}, got {actual}")]
    IdIndexMismatch { expected: u64, actual: u64 },
    #[error("unknown {type_name} variant: {value}")]
    UnknownVariant {
        type_name: &'static str,
        value: String,
    },
}

#[derive(Debug, Deserialize)]
struct AstDocumentEnvelope {
    schema_version: u32,
    document: AstDocumentDto,
}

#[derive(Debug, Deserialize)]
struct AstDocumentDto {
    structure: StructureAstDto,
    constraints: ConstraintSetDto,
}

#[derive(Debug, Deserialize)]
struct StructureAstDto {
    root: String,
    next_id: String,
    arena: Vec<Option<StructureNodeDto>>,
}

#[derive(Debug, Deserialize)]
struct StructureNodeDto {
    id: String,
    kind: NodeKindDto,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum NodeKindDto {
    Scalar {
        name: String,
    },
    Array {
        name: String,
        length: ExpressionDto,
    },
    Matrix {
        name: String,
        rows: ReferenceDto,
        cols: ReferenceDto,
    },
    Tuple {
        elements: Vec<String>,
    },
    Repeat {
        count: ExpressionDto,
        index_var: Option<String>,
        body: Vec<String>,
    },
    Section {
        header: Option<String>,
        body: Vec<String>,
    },
    Sequence {
        children: Vec<String>,
    },
    Choice {
        tag: ReferenceDto,
        variants: Vec<ChoiceVariantDto>,
    },
    Hole {
        expected_kind: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct ChoiceVariantDto {
    tag_value: LiteralDto,
    body: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ConstraintSetDto {
    next_id: String,
    arena: Vec<Option<ConstraintEntryDto>>,
    by_node: Vec<ByNodeEntryDto>,
    global: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ConstraintEntryDto {
    id: String,
    constraint: ConstraintDto,
}

#[derive(Debug, Deserialize)]
struct ByNodeEntryDto {
    node_id: String,
    constraints: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum ConstraintDto {
    Range {
        target: ReferenceDto,
        lower: ExpressionDto,
        upper: ExpressionDto,
    },
    TypeDecl {
        target: ReferenceDto,
        expected: String,
    },
    LengthRelation {
        target: ReferenceDto,
        length: ExpressionDto,
    },
    Relation {
        lhs: ExpressionDto,
        op: String,
        rhs: ExpressionDto,
    },
    Distinct {
        elements: ReferenceDto,
        unit: String,
    },
    Property {
        target: ReferenceDto,
        tag: PropertyTagDto,
    },
    SumBound {
        variable: ReferenceDto,
        upper: ExpressionDto,
    },
    Sorted {
        elements: ReferenceDto,
        order: String,
    },
    Guarantee {
        description: String,
        predicate: Option<ExpressionDto>,
    },
    CharSet {
        target: ReferenceDto,
        charset: CharSetSpecDto,
    },
    StringLength {
        target: ReferenceDto,
        min: ExpressionDto,
        max: ExpressionDto,
    },
    RenderHint {
        target: ReferenceDto,
        hint: RenderHintKindDto,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum ExpressionDto {
    Lit {
        value: i64,
    },
    Var {
        reference: ReferenceDto,
    },
    BinOp {
        op: String,
        lhs: Box<ExpressionDto>,
        rhs: Box<ExpressionDto>,
    },
    Pow {
        base: Box<ExpressionDto>,
        exp: Box<ExpressionDto>,
    },
    FnCall {
        name: String,
        args: Vec<ExpressionDto>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum ReferenceDto {
    VariableRef {
        node_id: String,
    },
    IndexedRef {
        target: String,
        indices: Vec<String>,
    },
    Unresolved {
        name: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum LiteralDto {
    IntLit { value: i64 },
    StrLit { value: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum PropertyTagDto {
    Simple,
    Connected,
    Tree,
    Permutation,
    Binary,
    Odd,
    Even,
    Custom { value: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum CharSetSpecDto {
    LowerAlpha,
    UpperAlpha,
    Alpha,
    Digit,
    AlphaNumeric,
    Custom { chars: Vec<char> },
    Range { from: char, to: char },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum RenderHintKindDto {
    Separator { value: String },
}

pub fn engine_from_json(json: &str) -> Result<AstEngine, AstDtoError> {
    let envelope: AstDocumentEnvelope = serde_json::from_str(json)?;
    envelope_to_engine(envelope)
}

fn envelope_to_engine(envelope: AstDocumentEnvelope) -> Result<AstEngine, AstDtoError> {
    if envelope.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(AstDtoError::UnsupportedVersion(envelope.schema_version));
    }

    Ok(AstEngine {
        structure: dto_to_structure(envelope.document.structure)?,
        constraints: dto_to_constraint_set(envelope.document.constraints)?,
    })
}

fn parse_u64(s: &str) -> Result<u64, AstDtoError> {
    s.parse::<u64>()
        .map_err(|_| AstDtoError::InvalidId(s.to_owned()))
}

fn parse_node_id(s: &str) -> Result<NodeId, AstDtoError> {
    parse_u64(s).map(NodeId::from_raw)
}

fn parse_constraint_id(s: &str) -> Result<ConstraintId, AstDtoError> {
    parse_u64(s).map(ConstraintId::from_raw)
}

fn parse_node_ids(ids: &[String]) -> Result<Vec<NodeId>, AstDtoError> {
    ids.iter().map(|s| parse_node_id(s)).collect()
}

fn dto_to_structure(dto: StructureAstDto) -> Result<StructureAst, AstDtoError> {
    let root = parse_node_id(&dto.root)?;
    let next_id = parse_u64(&dto.next_id)?;
    let mut arena = Vec::with_capacity(dto.arena.len());

    for (index, slot) in dto.arena.into_iter().enumerate() {
        match slot {
            None => arena.push(None),
            Some(node_dto) => {
                let id = parse_node_id(&node_dto.id)?;
                let expected =
                    u64::try_from(index).map_err(|_| AstDtoError::InvalidId(index.to_string()))?;
                if id.value() != expected {
                    return Err(AstDtoError::IdIndexMismatch {
                        expected,
                        actual: id.value(),
                    });
                }
                arena.push(Some(StructureNode::new(
                    id,
                    dto_to_node_kind(node_dto.kind)?,
                )));
            }
        }
    }

    Ok(StructureAst::from_raw_parts(root, arena, next_id))
}

fn dto_to_node_kind(dto: NodeKindDto) -> Result<NodeKind, AstDtoError> {
    match dto {
        NodeKindDto::Scalar { name } => Ok(NodeKind::Scalar {
            name: Ident::new(&name),
        }),
        NodeKindDto::Array { name, length } => Ok(NodeKind::Array {
            name: Ident::new(&name),
            length: dto_to_expr(length)?,
        }),
        NodeKindDto::Matrix { name, rows, cols } => Ok(NodeKind::Matrix {
            name: Ident::new(&name),
            rows: dto_to_ref(rows)?,
            cols: dto_to_ref(cols)?,
        }),
        NodeKindDto::Tuple { elements } => Ok(NodeKind::Tuple {
            elements: parse_node_ids(&elements)?,
        }),
        NodeKindDto::Repeat {
            count,
            index_var,
            body,
        } => Ok(NodeKind::Repeat {
            count: dto_to_expr(count)?,
            index_var: index_var.map(|s| Ident::new(&s)),
            body: parse_node_ids(&body)?,
        }),
        NodeKindDto::Section { header, body } => Ok(NodeKind::Section {
            header: header.map(|s| parse_node_id(&s)).transpose()?,
            body: parse_node_ids(&body)?,
        }),
        NodeKindDto::Sequence { children } => Ok(NodeKind::Sequence {
            children: parse_node_ids(&children)?,
        }),
        NodeKindDto::Choice { tag, variants } => {
            let variants = variants
                .into_iter()
                .map(|variant| {
                    Ok((
                        dto_to_literal(variant.tag_value),
                        parse_node_ids(&variant.body)?,
                    ))
                })
                .collect::<Result<Vec<_>, AstDtoError>>()?;
            Ok(NodeKind::Choice {
                tag: dto_to_ref(tag)?,
                variants,
            })
        }
        NodeKindDto::Hole { expected_kind } => Ok(NodeKind::Hole {
            expected_kind: expected_kind.map(|s| str_to_hint(&s)).transpose()?,
        }),
    }
}

fn dto_to_constraint_set(dto: ConstraintSetDto) -> Result<ConstraintSet, AstDtoError> {
    let next_id = parse_u64(&dto.next_id)?;
    let mut arena = Vec::with_capacity(dto.arena.len());

    for (index, slot) in dto.arena.into_iter().enumerate() {
        match slot {
            None => arena.push(None),
            Some(entry) => {
                let id = parse_constraint_id(&entry.id)?;
                let expected =
                    u64::try_from(index).map_err(|_| AstDtoError::InvalidId(index.to_string()))?;
                if id.value() != expected {
                    return Err(AstDtoError::IdIndexMismatch {
                        expected,
                        actual: id.value(),
                    });
                }
                arena.push(Some(dto_to_constraint(entry.constraint)?));
            }
        }
    }

    let by_node = dto
        .by_node
        .into_iter()
        .map(|entry| {
            let node_id = parse_node_id(&entry.node_id)?;
            let constraints = entry
                .constraints
                .iter()
                .map(|s| parse_constraint_id(s))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((node_id, constraints))
        })
        .collect::<Result<Vec<_>, AstDtoError>>()?;

    let global = dto
        .global
        .iter()
        .map(|s| parse_constraint_id(s))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ConstraintSet::from_raw_parts(
        arena, by_node, global, next_id,
    ))
}

fn dto_to_constraint(dto: ConstraintDto) -> Result<Constraint, AstDtoError> {
    match dto {
        ConstraintDto::Range {
            target,
            lower,
            upper,
        } => Ok(Constraint::Range {
            target: dto_to_ref(target)?,
            lower: dto_to_expr(lower)?,
            upper: dto_to_expr(upper)?,
        }),
        ConstraintDto::TypeDecl { target, expected } => Ok(Constraint::TypeDecl {
            target: dto_to_ref(target)?,
            expected: str_to_expected_type(&expected)?,
        }),
        ConstraintDto::LengthRelation { target, length } => Ok(Constraint::LengthRelation {
            target: dto_to_ref(target)?,
            length: dto_to_expr(length)?,
        }),
        ConstraintDto::Relation { lhs, op, rhs } => Ok(Constraint::Relation {
            lhs: dto_to_expr(lhs)?,
            op: str_to_relation_op(&op)?,
            rhs: dto_to_expr(rhs)?,
        }),
        ConstraintDto::Distinct { elements, unit } => Ok(Constraint::Distinct {
            elements: dto_to_ref(elements)?,
            unit: str_to_distinct_unit(&unit)?,
        }),
        ConstraintDto::Property { target, tag } => Ok(Constraint::Property {
            target: dto_to_ref(target)?,
            tag: dto_to_property_tag(tag),
        }),
        ConstraintDto::SumBound { variable, upper } => Ok(Constraint::SumBound {
            variable: dto_to_ref(variable)?,
            upper: dto_to_expr(upper)?,
        }),
        ConstraintDto::Sorted { elements, order } => Ok(Constraint::Sorted {
            elements: dto_to_ref(elements)?,
            order: str_to_sort_order(&order)?,
        }),
        ConstraintDto::Guarantee {
            description,
            predicate,
        } => Ok(Constraint::Guarantee {
            description,
            predicate: predicate.map(dto_to_expr).transpose()?,
        }),
        ConstraintDto::CharSet { target, charset } => Ok(Constraint::CharSet {
            target: dto_to_ref(target)?,
            charset: dto_to_charset(charset),
        }),
        ConstraintDto::StringLength { target, min, max } => Ok(Constraint::StringLength {
            target: dto_to_ref(target)?,
            min: dto_to_expr(min)?,
            max: dto_to_expr(max)?,
        }),
        ConstraintDto::RenderHint { target, hint } => Ok(Constraint::RenderHint {
            target: dto_to_ref(target)?,
            hint: dto_to_render_hint(hint)?,
        }),
    }
}

fn dto_to_expr(dto: ExpressionDto) -> Result<Expression, AstDtoError> {
    match dto {
        ExpressionDto::Lit { value } => Ok(Expression::Lit(value)),
        ExpressionDto::Var { reference } => Ok(Expression::Var(dto_to_ref(reference)?)),
        ExpressionDto::BinOp { op, lhs, rhs } => Ok(Expression::BinOp {
            op: str_to_arith_op(&op)?,
            lhs: Box::new(dto_to_expr(*lhs)?),
            rhs: Box::new(dto_to_expr(*rhs)?),
        }),
        ExpressionDto::Pow { base, exp } => Ok(Expression::Pow {
            base: Box::new(dto_to_expr(*base)?),
            exp: Box::new(dto_to_expr(*exp)?),
        }),
        ExpressionDto::FnCall { name, args } => Ok(Expression::FnCall {
            name: Ident::new(&name),
            args: args
                .into_iter()
                .map(dto_to_expr)
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn dto_to_ref(dto: ReferenceDto) -> Result<Reference, AstDtoError> {
    match dto {
        ReferenceDto::VariableRef { node_id } => {
            Ok(Reference::VariableRef(parse_node_id(&node_id)?))
        }
        ReferenceDto::IndexedRef { target, indices } => Ok(Reference::IndexedRef {
            target: parse_node_id(&target)?,
            indices: indices.iter().map(|s| Ident::new(s)).collect(),
        }),
        ReferenceDto::Unresolved { name } => Ok(Reference::Unresolved(Ident::new(&name))),
    }
}

fn dto_to_literal(dto: LiteralDto) -> Literal {
    match dto {
        LiteralDto::IntLit { value } => Literal::IntLit(value),
        LiteralDto::StrLit { value } => Literal::StrLit(value),
    }
}

fn dto_to_property_tag(dto: PropertyTagDto) -> PropertyTag {
    match dto {
        PropertyTagDto::Simple => PropertyTag::Simple,
        PropertyTagDto::Connected => PropertyTag::Connected,
        PropertyTagDto::Tree => PropertyTag::Tree,
        PropertyTagDto::Permutation => PropertyTag::Permutation,
        PropertyTagDto::Binary => PropertyTag::Binary,
        PropertyTagDto::Odd => PropertyTag::Odd,
        PropertyTagDto::Even => PropertyTag::Even,
        PropertyTagDto::Custom { value } => PropertyTag::Custom(value),
    }
}

fn dto_to_charset(dto: CharSetSpecDto) -> CharSetSpec {
    match dto {
        CharSetSpecDto::LowerAlpha => CharSetSpec::LowerAlpha,
        CharSetSpecDto::UpperAlpha => CharSetSpec::UpperAlpha,
        CharSetSpecDto::Alpha => CharSetSpec::Alpha,
        CharSetSpecDto::Digit => CharSetSpec::Digit,
        CharSetSpecDto::AlphaNumeric => CharSetSpec::AlphaNumeric,
        CharSetSpecDto::Custom { chars } => CharSetSpec::Custom(chars),
        CharSetSpecDto::Range { from, to } => CharSetSpec::Range(from, to),
    }
}

fn dto_to_render_hint(dto: RenderHintKindDto) -> Result<RenderHintKind, AstDtoError> {
    match dto {
        RenderHintKindDto::Separator { value } => {
            Ok(RenderHintKind::Separator(str_to_separator(&value)?))
        }
    }
}

fn str_to_expected_type(s: &str) -> Result<ExpectedType, AstDtoError> {
    match s {
        "Int" => Ok(ExpectedType::Int),
        "Str" => Ok(ExpectedType::Str),
        "Char" => Ok(ExpectedType::Char),
        _ => unknown("ExpectedType", s),
    }
}

fn str_to_relation_op(s: &str) -> Result<RelationOp, AstDtoError> {
    match s {
        "Lt" => Ok(RelationOp::Lt),
        "Le" => Ok(RelationOp::Le),
        "Gt" => Ok(RelationOp::Gt),
        "Ge" => Ok(RelationOp::Ge),
        "Eq" => Ok(RelationOp::Eq),
        "Ne" => Ok(RelationOp::Ne),
        _ => unknown("RelationOp", s),
    }
}

fn str_to_arith_op(s: &str) -> Result<ArithOp, AstDtoError> {
    match s {
        "Add" => Ok(ArithOp::Add),
        "Sub" => Ok(ArithOp::Sub),
        "Mul" => Ok(ArithOp::Mul),
        "Div" => Ok(ArithOp::Div),
        _ => unknown("ArithOp", s),
    }
}

fn str_to_distinct_unit(s: &str) -> Result<DistinctUnit, AstDtoError> {
    match s {
        "Element" => Ok(DistinctUnit::Element),
        "Tuple" => Ok(DistinctUnit::Tuple),
        _ => unknown("DistinctUnit", s),
    }
}

fn str_to_sort_order(s: &str) -> Result<SortOrder, AstDtoError> {
    match s {
        "Ascending" => Ok(SortOrder::Ascending),
        "NonDecreasing" => Ok(SortOrder::NonDecreasing),
        "Descending" => Ok(SortOrder::Descending),
        "NonIncreasing" => Ok(SortOrder::NonIncreasing),
        _ => unknown("SortOrder", s),
    }
}

fn str_to_separator(s: &str) -> Result<Separator, AstDtoError> {
    match s {
        "Space" => Ok(Separator::Space),
        "None" => Ok(Separator::None),
        _ => unknown("Separator", s),
    }
}

fn str_to_hint(s: &str) -> Result<NodeKindHint, AstDtoError> {
    match s {
        "AnyScalar" => Ok(NodeKindHint::AnyScalar),
        "AnyArray" => Ok(NodeKindHint::AnyArray),
        "AnyMatrix" => Ok(NodeKindHint::AnyMatrix),
        "AnyTuple" => Ok(NodeKindHint::AnyTuple),
        "AnyRepeat" => Ok(NodeKindHint::AnyRepeat),
        "AnySection" => Ok(NodeKindHint::AnySection),
        "AnyChoice" => Ok(NodeKindHint::AnyChoice),
        "Any" => Ok(NodeKindHint::Any),
        _ => unknown("NodeKindHint", s),
    }
}

fn unknown<T>(type_name: &'static str, value: &str) -> Result<T, AstDtoError> {
    Err(AstDtoError::UnknownVariant {
        type_name,
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_minimal_scalar_engine() {
        let json = r#"{
          "schema_version": 1,
          "document": {
            "structure": {
              "root": "0",
              "next_id": "2",
              "arena": [
                {"id":"0","kind":{"kind":"Sequence","children":["1"]}},
                {"id":"1","kind":{"kind":"Scalar","name":"N"}}
              ]
            },
            "constraints": {
              "next_id": "2",
              "arena": [
                {"id":"0","constraint":{"kind":"TypeDecl","target":{"kind":"VariableRef","node_id":"1"},"expected":"Int"}},
                {"id":"1","constraint":{"kind":"Range","target":{"kind":"VariableRef","node_id":"1"},"lower":{"kind":"Lit","value":1},"upper":{"kind":"Lit","value":3}}}
              ],
              "by_node": [{"node_id":"1","constraints":["0","1"]}],
              "global": []
            }
          }
        }"#;

        let engine = engine_from_json(json).unwrap();
        assert_eq!(engine.structure.len(), 2);
        assert_eq!(engine.constraints.len(), 2);
    }
}
