/**
 * @file Experimental grammar for a "system" language targetting WASL & WASL WC
 * @author Eric Kidd
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/** @type {function(RuleOrLiteral): Rule} */
function commaSep(rule) {
  return optional(seq(rule, repeat(seq(",", rule)), optional(",")));
}

export default grammar({
  name: "wasl",

  word: ($) => $.ident,

  extras: ($) => [
    /\s/, // whitespace
    $.comment,
  ],

  supertypes: ($) => [
    $._top_level,
    $._expr,
    $._atom,
    $._linear_val_type,
    $._linear_storage_type,
    $._val_type,
  ],

  rules: {
    source_file: ($) => repeat($._top_level),

    _top_level: ($) => choice($.import_block, $.func),

    import_block: ($) =>
      seq(
        "import",
        field("mod_name", $.ident),
        "{",
        repeat(seq(field("import", $.import_func), ";")),
        "}",
      ),

    import_func: ($) => $.func_sig,

    func: ($) =>
      seq(
        field("export", optional("export")),
        field("sig", $.func_sig),
        field("body", $.block),
      ),

    func_sig: ($) =>
      seq(
        "func",
        field("name", $.ident),
        field("params", $.params),
        optional(field("returns", $.returns)),
      ),

    params: ($) => seq("(", commaSep($.param), ")"),
    param: ($) => seq(field("name", $.ident), ":", field("type", $._val_type)),

    returns: ($) =>
      seq(
        "->",
        choice(
          field("single", $._val_type),
          seq("(", commaSep(field("multiple", $._val_type)), ")"),
        ),
      ),

    block: ($) => seq("{", field("expr", $._expr), "}"),

    // Anonymous rule with named children to force creation of an
    // `enum` in Rust `type-sitter` bindings.
    _expr: ($) => choice($.binop, $.call, $._atom),

    binop: ($) =>
      choice(
        prec.left(
          1,
          seq(
            field("left", $._expr),
            field("op", "+"),
            field("right", $._expr),
          ),
        ),
        prec.left(
          2,
          seq(
            field("left", $._expr),
            field("op", "*"),
            field("right", $._expr),
          ),
        ),
      ),

    call: ($) =>
      seq(
        field("func", $.ident),
        "(",
        seq(commaSep(field("arg", $._expr))),
        ")",
      ),

    _atom: ($) => choice($.number, $.ident, $.parenExpr),

    parenExpr: ($) => seq("(", field("expr", $._expr), ")"),

    _linear_val_type: ($) => choice("i32", "u32", $.ptr_type),
    _linear_storage_type: ($) =>
      choice("i8", "u8", $._linear_val_type, $.linear_record_type),
    ptr_type: ($) =>
      seq(
        "*",
        field("mut", optional("mut")),
        field("null", optional("null")),
        field("to_type", $._linear_storage_type),
      ),
    linear_record_type: ($) =>
      seq("record", "{", commaSep(field("field", $.linear_field)), "}"),
    linear_field: ($) =>
      seq(field("name", $.ident), ":", field("type", $._linear_storage_type)),

    _val_type: ($) => choice($._linear_val_type), // Will add ref types here.

    number: ($) => /\d+/,

    ident: ($) => /[_a-zA-Z][_a-zA-Z0-9]*/,

    comment: ($) => token(seq("//", /.*/)),
  },
});
