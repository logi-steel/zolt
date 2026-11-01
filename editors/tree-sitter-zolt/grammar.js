/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

// Gramatyka tree-sitter dla Zolta: tylko do kolorowania, wcięć i nawigacji w edytorze.
//
// Celowo jest "płaska", jak w OXEL-u: zna funkcje, szablony, bloki { }, wywołania,
// teksty, listy i pojedyncze tokeny, ale nie sprawdza instrukcji. Dzięki temu koloruje
// dobrze także plik w trakcie pisania. Prawdziwe sprawdzanie programu robi `zolt lsp`.

module.exports = grammar({
  name: 'zolt',

  extras: $ => [/\s/, $.comment],

  // słowa kluczowe są wycinane z identyfikatorów (let, if, loop...)
  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat(choice($.function_definition, $.template)),

    _item: $ => choice(
      $.block,
      $.call,
      $.scoped,
      $.member,
      $.method_call,
      $.typed_number,
      $.number,
      $.boolean,
      $.string,
      $.list,
      $.identifier,
      'let', 'if', 'else', 'elseif', 'while', 'has', 'retrn',
      'loop', 'times', 'inf', 'end', 'lst',
      '&_', '&',
      ':', '=', '<', '>', '<=', '>=', '+', '-', '*', '/', '%', '<>', '->',
      '.', ',', '(', ')',
    ),

    // fn dodaj (a, &_b use => std::io) { ... }
    function_definition: $ => seq(
      'fn',
      field('name', $.identifier),
      field('parameters', $.parameters),
      field('body', $.block),
    ),

    parameters: $ => seq('(', repeat(choice($.parameter, ',', $.use_clause)), ')'),

    parameter: $ => seq(
      optional(choice('&_', '&')),
      field('name', $.identifier),
      optional(seq('.', 'lst')),
    ),

    use_clause: $ => prec.right(seq('use', '=>', repeat(choice($.scoped, ',')))),

    // gracz : { hp : 30  pokaz : (use => std::io) { ... } }
    template: $ => seq(
      field('name', $.identifier),
      ':',
      field('body', $.template_body),
    ),

    template_body: $ => seq('{', repeat(choice($.field_definition, $.method_definition)), '}'),

    field_definition: $ => seq(field('name', $.identifier), ':', $._value),

    _value: $ => choice(
      seq(optional('-'), choice($.number, $.typed_number)),
      $.boolean,
      $.string,
      $.list,
    ),

    method_definition: $ => seq(
      field('name', $.identifier),
      ':',
      optional(field('parameters', $.parameters)),
      field('body', $.block),
    ),

    block: $ => seq('{', repeat($._item), '}'),

    // nazwa tuż przed `(`; argumenty to zwykłe tokeny, które idą dalej
    call: $ => prec(1, seq(field('function', $.identifier), '(')),

    // sys::out, sys::read2, std::io
    scoped: $ => prec(2, seq(
      field('scope', $.identifier),
      repeat1(seq('::', field('name', $.identifier))),
    )),

    // gracz.1.hp, spoczko.lst.add
    member: $ => prec(1, seq('.', field('name', $.identifier))),

    // gracz.1.pokaz()
    method_call: $ => prec(2, seq('.', field('name', $.identifier), '(')),

    identifier: _ => /[\p{L}_][\p{L}\p{N}_]*/,

    // c32_3.14, c8_100: rozmiar w bitach przyklejony do wartości
    typed_number: _ => token(prec(1, /c[0-9]+_[0-9]+(\.[0-9]+)?/)),

    number: _ => /[0-9]+(\.[0-9]+)?/,

    boolean: _ => choice('true', 'false', token(prec(1, /b\.(true|false)/))),

    string: $ => seq(
      '"',
      repeat(choice($.string_content, $.escape_sequence, $.interpolation)),
      token.immediate('"'),
    ),

    string_content: _ => token.immediate(prec(1, /[^"\\<]+/)),

    escape_sequence: _ => token.immediate(/\\./),

    // <x> w tekście wstawia wartość
    interpolation: _ => token.immediate(prec(2, /<[^>"]*>/)),

    // [oki, baj, po co, 12]: gołe słowa to tekst
    list: $ => seq('[', optional(seq($.list_item, repeat(seq(',', $.list_item)))), ']'),

    list_item: _ => /[^\[\],\n]+/,

    comment: _ => token(seq('__', /([^_]|_[^_])*/, '__')),
  },
});
