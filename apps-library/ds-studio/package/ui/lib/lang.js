/* Notebook language for the JavaScript kernel.
 *
 * The App sandbox serves a CSP without 'unsafe-eval', so neither eval() nor
 * new Function() is available: cell code is tokenised, parsed and walked here
 * instead of handed to the JavaScript engine. The surface is deliberately
 * pandas-shaped, because the agent writing the cells has seen a lot of pandas.
 *
 *   v = load("ventes")
 *   top = v.filter(annee >= 2020).groupby("produit").agg(total = sum(montant))
 *   bar(top.sort("total", desc = true).head(10), x = "produit", y = "total")
 */
(function(root, factory){
  'use strict';
  var api = factory(
    typeof module === 'object' && module.exports ? require('./frame.js') : root.StudioFrame,
    typeof module === 'object' && module.exports ? require('./csv.js') : root.StudioCsv
  );
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else {
    root.StudioLang = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function(Frames, Csv){
  'use strict';

  var Frame = Frames.Frame;
  var Grouped = Frames.Grouped;

  /* ------------------------------------------------------------- Errors */

  function StudioError(message, line, column, hint) {
    var error = new Error(message);
    error.name = 'StudioError';
    error.line = line || 0;
    error.column = column || 0;
    error.hint = hint || '';
    return error;
  }

  function at(node) {
    return node && node.line ? node : { line: 0, column: 0 };
  }

  function fail(node, message, hint) {
    var position = at(node);
    throw StudioError(message, position.line, position.column, hint);
  }

  /* ---------------------------------------------------------- Tokenizer */

  var KEYWORDS = {
    'true': 'true', 'false': 'false', 'null': 'null',
    'and': 'and', 'or': 'or', 'not': 'not'
  };

  var OPERATORS = [
    '==', '!=', '<=', '>=', '&&', '||',
    '=', '<', '>', '+', '-', '*', '/', '%', '!',
    '(', ')', '[', ']', ',', '.', ':'
  ];

  function tokenize(source) {
    var tokens = [];
    var line = 1;
    var column = 1;
    var i = 0;
    var text = String(source);

    function push(type, value, startLine, startColumn) {
      tokens.push({ type: type, value: value, line: startLine, column: startColumn });
    }

    while (i < text.length) {
      var char = text[i];

      if (char === '\r') { i += 1; continue; }

      if (char === '\n') {
        push('newline', '\n', line, column);
        i += 1;
        line += 1;
        column = 1;
        continue;
      }

      if (char === ' ' || char === '\t') { i += 1; column += 1; continue; }

      if (char === '#') {
        while (i < text.length && text[i] !== '\n') { i += 1; column += 1; }
        continue;
      }

      if (char === '"' || char === "'") {
        var quote = char;
        var startLine = line;
        var startColumn = column;
        var value = '';
        i += 1;
        column += 1;
        var closed = false;
        while (i < text.length) {
          var current = text[i];
          if (current === '\\' && i + 1 < text.length) {
            var next = text[i + 1];
            value += next === 'n' ? '\n' : next === 't' ? '\t' : next;
            i += 2;
            column += 2;
            continue;
          }
          if (current === quote) { closed = true; i += 1; column += 1; break; }
          if (current === '\n') { line += 1; column = 0; }
          value += current;
          i += 1;
          column += 1;
        }
        if (!closed) {
          throw StudioError('unterminated text value', startLine, startColumn, 'add the closing ' + quote);
        }
        push('string', value, startLine, startColumn);
        continue;
      }

      if (char >= '0' && char <= '9') {
        var numberStart = column;
        var raw = '';
        while (i < text.length && /[0-9_]/.test(text[i])) { raw += text[i]; i += 1; column += 1; }
        if (text[i] === '.' && /[0-9]/.test(text[i + 1] || '')) {
          raw += '.';
          i += 1;
          column += 1;
          while (i < text.length && /[0-9_]/.test(text[i])) { raw += text[i]; i += 1; column += 1; }
        }
        if (text[i] === 'e' || text[i] === 'E') {
          var save = i;
          var exponent = text[i];
          var cursor = i + 1;
          if (text[cursor] === '+' || text[cursor] === '-') { exponent += text[cursor]; cursor += 1; }
          if (/[0-9]/.test(text[cursor] || '')) {
            while (cursor < text.length && /[0-9]/.test(text[cursor])) { exponent += text[cursor]; cursor += 1; }
            raw += exponent;
            column += cursor - i;
            i = cursor;
          } else {
            i = save;
          }
        }
        push('number', Number(raw.replace(/_/g, '')), line, numberStart);
        continue;
      }

      if (/[A-Za-z_À-ɏ]/.test(char)) {
        var identStart = column;
        var name = '';
        while (i < text.length && /[A-Za-z0-9_À-ɏ]/.test(text[i])) {
          name += text[i];
          i += 1;
          column += 1;
        }
        push(KEYWORDS[name] || 'ident', name, line, identStart);
        continue;
      }

      var matched = null;
      for (var o = 0; o < OPERATORS.length; o += 1) {
        if (text.substr(i, OPERATORS[o].length) === OPERATORS[o]) { matched = OPERATORS[o]; break; }
      }
      if (!matched) {
        throw StudioError('unexpected character "' + char + '"', line, column);
      }
      push(matched, matched, line, column);
      i += matched.length;
      column += matched.length;
    }

    push('eof', null, line, column);
    return joinContinuations(tokens);
  }

  /* A newline ends a statement unless the line clearly continues: it stopped
   * on an operator, or the next line opens with a chained method call. */
  var CONTINUING = {
    '=': true, '<': true, '>': true, '+': true, '-': true, '*': true, '/': true,
    '%': true, '!': true, '(': true, '[': true, ',': true, '.': true, ':': true,
    '==': true, '!=': true, '<=': true, '>=': true, '&&': true, '||': true,
    'and': true, 'or': true, 'not': true
  };

  function joinContinuations(tokens) {
    var out = [];
    for (var i = 0; i < tokens.length; i += 1) {
      var token = tokens[i];
      if (token.type !== 'newline') { out.push(token); continue; }
      var previous = out[out.length - 1];
      if (!previous || previous.type === 'newline') continue;
      if (CONTINUING[previous.type]) continue;
      var next = tokens[i + 1];
      while (next && next.type === 'newline') { i += 1; next = tokens[i + 1]; }
      if (next && (next.type === '.' || next.type === ')' || next.type === ']' || next.type === ',')) continue;
      out.push(token);
    }
    return out;
  }

  /* ------------------------------------------------------------- Parser */

  function Parser(tokens) {
    this.tokens = tokens;
    this.position = 0;
  }

  Parser.prototype.peek = function(offset) {
    return this.tokens[this.position + (offset || 0)] || this.tokens[this.tokens.length - 1];
  };

  Parser.prototype.next = function() {
    var token = this.peek();
    if (token.type !== 'eof') this.position += 1;
    return token;
  };

  Parser.prototype.check = function(type) {
    return this.peek().type === type;
  };

  Parser.prototype.accept = function(type) {
    if (this.check(type)) return this.next();
    return null;
  };

  Parser.prototype.expect = function(type, what) {
    if (this.check(type)) return this.next();
    var token = this.peek();
    throw StudioError(
      'expected ' + (what || '"' + type + '"') + ' but found ' + describe(token),
      token.line,
      token.column
    );
  };

  function describe(token) {
    if (token.type === 'eof') return 'the end of the cell';
    if (token.type === 'newline') return 'the end of the line';
    if (token.type === 'string') return 'text "' + token.value + '"';
    return '"' + String(token.value) + '"';
  }

  Parser.prototype.skipNewlines = function() {
    while (this.check('newline')) this.next();
  };

  Parser.prototype.parseProgram = function() {
    var statements = [];
    this.skipNewlines();
    while (!this.check('eof')) {
      statements.push(this.parseStatement());
      if (!this.check('eof')) {
        this.expect('newline', 'the end of the statement');
      }
      this.skipNewlines();
    }
    return { type: 'Program', body: statements };
  };

  Parser.prototype.parseStatement = function() {
    if (this.check('ident') && this.peek(1).type === '=' && this.peek(2).type !== '=') {
      var name = this.next();
      this.next();
      var value = this.parseExpression();
      return { type: 'Assign', name: name.value, value: value, line: name.line, column: name.column };
    }
    var expression = this.parseExpression();
    return { type: 'Expression', value: expression, line: expression.line, column: expression.column };
  };

  Parser.prototype.parseExpression = function() {
    return this.parseOr();
  };

  Parser.prototype.parseOr = function() {
    var left = this.parseAnd();
    while (this.check('||') || this.check('or')) {
      var operator = this.next();
      var right = this.parseAnd();
      left = { type: 'Binary', op: '||', left: left, right: right, line: operator.line, column: operator.column };
    }
    return left;
  };

  Parser.prototype.parseAnd = function() {
    var left = this.parseEquality();
    while (this.check('&&') || this.check('and')) {
      var operator = this.next();
      var right = this.parseEquality();
      left = { type: 'Binary', op: '&&', left: left, right: right, line: operator.line, column: operator.column };
    }
    return left;
  };

  Parser.prototype.parseEquality = function() {
    var left = this.parseComparison();
    while (this.check('==') || this.check('!=')) {
      var operator = this.next();
      var right = this.parseComparison();
      left = { type: 'Binary', op: operator.type, left: left, right: right, line: operator.line, column: operator.column };
    }
    return left;
  };

  Parser.prototype.parseComparison = function() {
    var left = this.parseAdditive();
    while (this.check('<') || this.check('<=') || this.check('>') || this.check('>=')) {
      var operator = this.next();
      var right = this.parseAdditive();
      left = { type: 'Binary', op: operator.type, left: left, right: right, line: operator.line, column: operator.column };
    }
    return left;
  };

  Parser.prototype.parseAdditive = function() {
    var left = this.parseMultiplicative();
    while (this.check('+') || this.check('-')) {
      var operator = this.next();
      var right = this.parseMultiplicative();
      left = { type: 'Binary', op: operator.type, left: left, right: right, line: operator.line, column: operator.column };
    }
    return left;
  };

  Parser.prototype.parseMultiplicative = function() {
    var left = this.parseUnary();
    while (this.check('*') || this.check('/') || this.check('%')) {
      var operator = this.next();
      var right = this.parseUnary();
      left = { type: 'Binary', op: operator.type, left: left, right: right, line: operator.line, column: operator.column };
    }
    return left;
  };

  Parser.prototype.parseUnary = function() {
    if (this.check('-') || this.check('!') || this.check('not')) {
      var operator = this.next();
      var argument = this.parseUnary();
      var op = operator.type === '-' ? '-' : '!';
      return { type: 'Unary', op: op, argument: argument, line: operator.line, column: operator.column };
    }
    return this.parsePostfix();
  };

  Parser.prototype.parsePostfix = function() {
    var target = this.parsePrimary();
    for (;;) {
      if (this.check('.')) {
        var dot = this.next();
        var name = this.expect('ident', 'a method name');
        var call = this.parseArguments();
        target = {
          type: 'Method',
          target: target,
          name: name.value,
          args: call.args,
          named: call.named,
          namedOrder: call.namedOrder,
          line: name.line,
          column: name.column
        };
        continue;
      }
      if (this.check('[')) {
        var bracket = this.next();
        var index = this.parseExpression();
        this.expect(']');
        target = { type: 'Index', target: target, index: index, line: bracket.line, column: bracket.column };
        continue;
      }
      return target;
    }
  };

  Parser.prototype.parseArguments = function() {
    this.expect('(', 'an opening parenthesis');
    var args = [];
    var named = {};
    var namedOrder = [];
    this.skipNewlines();
    if (this.accept(')')) return { args: args, named: named, namedOrder: namedOrder };

    for (;;) {
      this.skipNewlines();
      if (this.check('ident') && this.peek(1).type === '=' && this.peek(2).type !== '=') {
        var key = this.next();
        this.next();
        var value = this.parseExpression();
        if (Object.prototype.hasOwnProperty.call(named, key.value)) {
          throw StudioError('argument "' + key.value + '" is given twice', key.line, key.column);
        }
        named[key.value] = value;
        namedOrder.push(key.value);
      } else {
        if (namedOrder.length) {
          var token = this.peek();
          throw StudioError(
            'a positional argument cannot follow a named argument',
            token.line,
            token.column,
            'name this argument too, or move it before "' + namedOrder[0] + ' ="'
          );
        }
        args.push(this.parseExpression());
      }
      this.skipNewlines();
      if (this.accept(',')) continue;
      this.expect(')', 'a closing parenthesis');
      return { args: args, named: named, namedOrder: namedOrder };
    }
  };

  Parser.prototype.parsePrimary = function() {
    var token = this.peek();

    if (token.type === 'number') { this.next(); return { type: 'Literal', value: token.value, line: token.line, column: token.column }; }
    if (token.type === 'string') { this.next(); return { type: 'Literal', value: token.value, line: token.line, column: token.column }; }
    if (token.type === 'true') { this.next(); return { type: 'Literal', value: true, line: token.line, column: token.column }; }
    if (token.type === 'false') { this.next(); return { type: 'Literal', value: false, line: token.line, column: token.column }; }
    if (token.type === 'null') { this.next(); return { type: 'Literal', value: null, line: token.line, column: token.column }; }

    if (token.type === '(') {
      this.next();
      this.skipNewlines();
      var inner = this.parseExpression();
      this.skipNewlines();
      this.expect(')');
      return inner;
    }

    if (token.type === '[') {
      this.next();
      var items = [];
      this.skipNewlines();
      if (!this.accept(']')) {
        for (;;) {
          this.skipNewlines();
          items.push(this.parseExpression());
          this.skipNewlines();
          if (this.accept(',')) continue;
          this.expect(']');
          break;
        }
      }
      return { type: 'List', items: items, line: token.line, column: token.column };
    }

    if (token.type === 'ident') {
      this.next();
      if (this.check('(')) {
        var call = this.parseArguments();
        return {
          type: 'Call',
          callee: token.value,
          args: call.args,
          named: call.named,
          namedOrder: call.namedOrder,
          line: token.line,
          column: token.column
        };
      }
      return { type: 'Identifier', name: token.value, line: token.line, column: token.column };
    }

    throw StudioError('unexpected ' + describe(token), token.line, token.column);
  };

  function parse(source) {
    return new Parser(tokenize(source)).parseProgram();
  }

  /* -------------------------------------------------------- Value model */

  function isFrame(value) { return value instanceof Frame; }
  function isGrouped(value) { return value instanceof Grouped; }

  function typeName(value) {
    if (value === null || value === undefined) return 'null';
    if (isFrame(value)) return 'table';
    if (isGrouped(value)) return 'grouped table';
    if (Array.isArray(value)) return 'list';
    if (value instanceof Date) return 'date';
    return typeof value;
  }

  function truthy(value) {
    if (value === null || value === undefined) return false;
    if (typeof value === 'boolean') return value;
    if (typeof value === 'number') return value !== 0 && !isNaN(value);
    if (typeof value === 'string') return value.length > 0;
    if (Array.isArray(value)) return value.length > 0;
    if (isFrame(value)) return value.length > 0;
    return true;
  }

  function equals(a, b) {
    if (a instanceof Date && b instanceof Date) return a.getTime() === b.getTime();
    if (a instanceof Date && typeof b === 'string') return a.toISOString().slice(0, 10) === b.slice(0, 10);
    if (b instanceof Date && typeof a === 'string') return b.toISOString().slice(0, 10) === a.slice(0, 10);
    if (a === null || a === undefined) return b === null || b === undefined;
    return a === b;
  }

  function toNumber(value, node, what) {
    if (typeof value === 'number') return value;
    if (typeof value === 'boolean') return value ? 1 : 0;
    if (value instanceof Date) return value.getTime();
    if (typeof value === 'string') {
      var parsed = Number(value.trim().replace(',', '.'));
      if (!isNaN(parsed)) return parsed;
    }
    if (value === null || value === undefined) return NaN;
    fail(node, (what || 'this value') + ' is a ' + typeName(value) + ', a number was expected');
    return NaN;
  }

  function toText(value) {
    if (value === null || value === undefined) return '';
    if (value instanceof Date) return value.toISOString();
    if (isFrame(value)) return 'table(' + value.length + ' x ' + value.columns.length + ')';
    if (Array.isArray(value)) return '[' + value.map(toText).join(', ') + ']';
    if (typeof value === 'number') {
      return Number.isInteger(value) ? String(value) : String(Math.round(value * 1e6) / 1e6);
    }
    return String(value);
  }

  /* ------------------------------------------------------- Aggregations */

  var AGG_ALIASES = {
    count: 'count',
    countDistinct: 'countDistinct',
    sum: 'sum',
    mean: 'mean',
    avg: 'mean',
    min: 'min',
    max: 'max',
    median: 'median',
    quantile: 'quantile',
    std: 'std',
    variance: 'variance',
    first: 'first',
    last: 'last',
    mode: 'mode'
  };

  /* ---------------------------------------------------------- Row scope */

  function RowScope(frame, parent) {
    this.frame = frame;
    this.parent = parent;
    this.index = 0;
  }

  RowScope.prototype.lookup = function(name) {
    if (this.frame.has(name)) {
      return { found: true, value: this.frame.data[name][this.index] };
    }
    return this.parent.lookup(name);
  };

  function Scope(variables, parent) {
    this.variables = variables || Object.create(null);
    this.parent = parent || null;
  }

  Scope.prototype.lookup = function(name) {
    if (Object.prototype.hasOwnProperty.call(this.variables, name)) {
      return { found: true, value: this.variables[name] };
    }
    if (this.parent) return this.parent.lookup(name);
    return { found: false };
  };

  Scope.prototype.set = function(name, value) {
    this.variables[name] = value;
  };

  /* ------------------------------------------------------- Interpreter */

  function Interpreter(options) {
    var settings = options || {};
    this.scope = new Scope();
    this.datasets = settings.datasets || Object.create(null);
    this.emit = settings.emit || function(){};
    this.deadline = settings.deadline || 0;
    this.token = settings.token || { cancelled: false };
    this.maxRows = settings.maxRows || 200000;
    this.steps = 0;
    this.checkEvery = 4000;
  }

  /* tick() counts evaluation steps and samples the budget; checkBudget() is
   * the unconditional test, called directly by long internal loops that do not
   * go through evaluate(). */
  Interpreter.prototype.tick = function(node) {
    this.steps += 1;
    if (this.steps !== 1 && this.steps % this.checkEvery !== 0) return;
    this.checkBudget(node);
  };

  Interpreter.prototype.checkBudget = function(node) {
    if (this.token.cancelled) {
      var cancelled = StudioError('execution cancelled', at(node).line, at(node).column);
      cancelled.cancelled = true;
      throw cancelled;
    }
    if (this.deadline && Date.now() > this.deadline) {
      var expired = StudioError(
        'execution stopped after the time budget',
        at(node).line,
        at(node).column,
        'reduce the number of rows, or split the cell'
      );
      expired.timeout = true;
      throw expired;
    }
  };

  Interpreter.prototype.run = function(program) {
    var last = { has: false, value: null };
    for (var i = 0; i < program.body.length; i += 1) {
      var statement = program.body[i];
      this.tick(statement);
      if (statement.type === 'Assign') {
        var value = this.evaluate(statement.value, this.scope);
        this.scope.set(statement.name, value);
        last = { has: false, value: null };
      } else {
        var result = this.evaluate(statement.value, this.scope);
        last = { has: result !== undefined, value: result === undefined ? null : result };
      }
    }
    return last;
  };

  Interpreter.prototype.evaluate = function(node, scope) {
    this.tick(node);
    switch (node.type) {
      case 'Literal':
        return node.value;
      case 'List':
        var self = this;
        return node.items.map(function(item){ return self.evaluate(item, scope); });
      case 'Identifier':
        var found = scope.lookup(node.name);
        if (found.found) return found.value;
        if (Object.prototype.hasOwnProperty.call(this.datasets, node.name)) {
          return this.datasets[node.name].frame;
        }
        fail(node, '"' + node.name + '" is not defined', this.nameHint(node.name, scope));
        return null;
      case 'Unary':
        return this.evaluateUnary(node, scope);
      case 'Binary':
        return this.evaluateBinary(node, scope);
      case 'Index':
        return this.evaluateIndex(node, scope);
      case 'Call':
        return this.callBuiltin(node, scope);
      case 'Method':
        return this.callMethod(node, scope);
      default:
        fail(node, 'unsupported expression');
        return null;
    }
  };

  Interpreter.prototype.nameHint = function(name, scope) {
    var candidates = [];
    var cursor = scope;
    while (cursor) {
      if (cursor.frame) candidates = candidates.concat(cursor.frame.columns);
      if (cursor.variables) candidates = candidates.concat(Object.keys(cursor.variables));
      cursor = cursor.parent;
    }
    candidates = candidates.concat(Object.keys(this.datasets), Object.keys(BUILTINS));
    var lower = name.toLowerCase();
    var close = candidates.filter(function(candidate){
      var other = candidate.toLowerCase();
      return other !== lower && (other.indexOf(lower) === 0 || lower.indexOf(other) === 0 ||
        Math.abs(other.length - lower.length) <= 2 && other.slice(0, 3) === lower.slice(0, 3));
    });
    return close.length ? 'did you mean ' + close.slice(0, 3).map(function(c){ return '"' + c + '"'; }).join(', ') + ' ?' : '';
  };

  Interpreter.prototype.evaluateUnary = function(node, scope) {
    var value = this.evaluate(node.argument, scope);
    if (node.op === '!') return !truthy(value);
    if (value === null || value === undefined) return null;
    return -toNumber(value, node.argument);
  };

  Interpreter.prototype.evaluateBinary = function(node, scope) {
    var op = node.op;

    if (op === '&&') {
      return truthy(this.evaluate(node.left, scope)) ? truthy(this.evaluate(node.right, scope)) : false;
    }
    if (op === '||') {
      return truthy(this.evaluate(node.left, scope)) ? true : truthy(this.evaluate(node.right, scope));
    }

    var left = this.evaluate(node.left, scope);
    var right = this.evaluate(node.right, scope);

    if (op === '==') return equals(left, right);
    if (op === '!=') return !equals(left, right);

    if (op === '+') {
      if (typeof left === 'string' || typeof right === 'string') return toText(left) + toText(right);
      if (Array.isArray(left) && Array.isArray(right)) return left.concat(right);
    }

    if (op === '<' || op === '<=' || op === '>' || op === '>=') {
      if (left === null || left === undefined || right === null || right === undefined) return false;
      var comparison = Frames.compareValues(left, right);
      if (op === '<') return comparison < 0;
      if (op === '<=') return comparison <= 0;
      if (op === '>') return comparison > 0;
      return comparison >= 0;
    }

    if (left === null || left === undefined || right === null || right === undefined) return null;

    var a = toNumber(left, node.left, 'the left operand');
    var b = toNumber(right, node.right, 'the right operand');
    if (op === '+') return a + b;
    if (op === '-') return a - b;
    if (op === '*') return a * b;
    if (op === '/') return b === 0 ? null : a / b;
    if (op === '%') return b === 0 ? null : a % b;
    fail(node, 'unsupported operator "' + op + '"');
    return null;
  };

  Interpreter.prototype.evaluateIndex = function(node, scope) {
    var target = this.evaluate(node.target, scope);
    var key = this.evaluate(node.index, scope);
    if (isFrame(target)) {
      if (typeof key === 'string') return target.col(key);
      if (typeof key === 'number') return target.row(key);
      fail(node, 'a table is indexed by a column name or a row number');
    }
    if (Array.isArray(target)) {
      var position = Math.trunc(toNumber(key, node.index, 'the index'));
      if (position < 0) position += target.length;
      return target[position] === undefined ? null : target[position];
    }
    if (target && typeof target === 'object') {
      var value = target[String(key)];
      return value === undefined ? null : value;
    }
    fail(node, 'cannot index a ' + typeName(target));
    return null;
  };

  /* Positional and named arguments resolved against a declared signature. */
  Interpreter.prototype.readArguments = function(node, scope, signature) {
    var self = this;
    var out = Object.create(null);
    var names = signature.map(function(entry){ return entry.name; });

    Object.keys(node.named).forEach(function(name){
      if (names.indexOf(name) === -1) {
        fail(node.named[name], 'unknown argument "' + name + '"',
          'accepted: ' + names.join(', '));
      }
    });

    node.args.forEach(function(argument, position){
      if (position >= signature.length) {
        fail(argument, 'too many arguments, ' + signature.length + ' accepted');
      }
      out[signature[position].name] = self.evaluate(argument, scope);
    });

    signature.forEach(function(entry, position){
      var provided = Object.prototype.hasOwnProperty.call(node.named, entry.name);
      if (provided) {
        if (position < node.args.length) {
          fail(node.named[entry.name], 'argument "' + entry.name + '" is already given by position');
        }
        out[entry.name] = self.evaluate(node.named[entry.name], scope);
        return;
      }
      if (Object.prototype.hasOwnProperty.call(out, entry.name)) return;
      if (entry.required) {
        fail(node, 'argument "' + entry.name + '" is required',
          'signature: ' + signature.map(describeParameter).join(', '));
      }
      out[entry.name] = entry.fallback === undefined ? null : entry.fallback;
    });

    return out;
  };

  function describeParameter(entry) {
    return entry.required ? entry.name : entry.name + ' = ' + JSON.stringify(entry.fallback === undefined ? null : entry.fallback);
  }

  /* Evaluates one expression once per row, with the row's columns in scope. */
  Interpreter.prototype.perRow = function(expression, frame, scope) {
    var rowScope = new RowScope(frame, scope);
    var out = new Array(frame.length);
    for (var i = 0; i < frame.length; i += 1) {
      rowScope.index = i;
      out[i] = this.evaluate(expression, rowScope);
    }
    return out;
  };

  Interpreter.prototype.expectFrame = function(value, node, what) {
    if (isFrame(value)) return value;
    fail(node, (what || 'this value') + ' is a ' + typeName(value) + ', a table was expected');
    return null;
  };

  Interpreter.prototype.expectColumn = function(frame, name, node) {
    if (typeof name !== 'string') {
      fail(node, 'a column name must be text, for example "' + (frame.columns[0] || 'column') + '"');
    }
    if (!frame.has(name)) {
      fail(node, 'unknown column "' + name + '"', 'available: ' + frame.columns.join(', '));
    }
    return name;
  };

  Interpreter.prototype.columnList = function(frame, value, node) {
    var self = this;
    var names = Array.isArray(value) ? value : [value];
    return names.map(function(name){ return self.expectColumn(frame, name, node); });
  };

  /* ------------------------------------------------------ agg specs */

  Interpreter.prototype.buildAggSpecs = function(node, frame, scope) {
    var self = this;
    var specs = [];

    if (!node.namedOrder.length) {
      fail(node, 'agg expects named results, for example agg(total = sum(amount))');
    }

    node.namedOrder.forEach(function(out){
      var call = node.named[out];
      if (call.type !== 'Call' || !AGG_ALIASES[call.callee]) {
        fail(call, 'agg expects an aggregation such as sum, mean, count, min, max, median, std',
          'write ' + out + ' = sum(column)');
      }
      var fn = AGG_ALIASES[call.callee];

      if (fn === 'count' && call.args.length === 0) {
        specs.push({ out: out, fn: 'count', values: null });
        return;
      }
      if (call.args.length === 0) {
        fail(call, '"' + call.callee + '" needs a column or an expression');
      }

      var computed = self.perRow(call.args[0], frame, scope);
      var argument;
      if (fn === 'quantile') {
        argument = call.args.length > 1
          ? toNumber(self.evaluate(call.args[1], scope), call.args[1], 'the quantile')
          : 0.5;
      }
      specs.push({
        out: out,
        fn: fn === 'count' ? 'countValid' : fn,
        arg: argument,
        values: function(indices){
          var picked = new Array(indices.length);
          for (var i = 0; i < indices.length; i += 1) picked[i] = computed[indices[i]];
          return picked;
        }
      });
    });

    return specs;
  };

  /* ------------------------------------------------------ Frame methods */

  var ROW_SCOPED = { filter: true, where: true, withColumn: true, assign: true, sortBy: true, agg: true, drop: false };

  Interpreter.prototype.callMethod = function(node, scope) {
    var target = this.evaluate(node.target, scope);

    if (isGrouped(target)) return this.groupedMethod(node, target, scope);
    if (isFrame(target)) return this.frameMethod(node, target, scope);
    if (Array.isArray(target)) return this.listMethod(node, target, scope);
    if (typeof target === 'string') return this.textMethod(node, target, scope);

    fail(node, 'a ' + typeName(target) + ' has no method "' + node.name + '"');
    return null;
  };

  Interpreter.prototype.groupedMethod = function(node, grouped, scope) {
    var name = node.name;
    if (name === 'agg') {
      return grouped.agg(this.buildAggSpecs(node, grouped.frame, scope));
    }
    if (name === 'count' || name === 'size') {
      return grouped.size();
    }
    fail(node, 'a grouped table has no method "' + name + '"', 'available: agg, count');
    return null;
  };

  Interpreter.prototype.frameMethod = function(node, frame, scope) {
    var self = this;
    var name = node.name;

    if (ROW_SCOPED[name]) {
      switch (name) {
        case 'filter':
        case 'where': {
          if (node.args.length !== 1 || node.namedOrder.length) {
            fail(node, 'filter expects one condition, for example filter(amount > 100)');
          }
          var keep = [];
          var rowScope = new RowScope(frame, scope);
          for (var i = 0; i < frame.length; i += 1) {
            rowScope.index = i;
            if (truthy(this.evaluate(node.args[0], rowScope))) keep.push(i);
          }
          return frame.take(keep);
        }
        case 'withColumn': {
          if (node.args.length !== 2) {
            fail(node, 'withColumn expects a name and an expression, for example withColumn("margin", price - cost)');
          }
          var column = this.evaluate(node.args[0], scope);
          if (typeof column !== 'string') fail(node.args[0], 'the new column name must be text');
          var values = this.perRow(node.args[1], frame, scope);
          return frame.withColumn(column, function(row, index){ return values[index]; });
        }
        case 'assign': {
          if (!node.namedOrder.length) {
            fail(node, 'assign expects named columns, for example assign(margin = price - cost)');
          }
          var result = frame;
          node.namedOrder.forEach(function(key){
            var computed = self.perRow(node.named[key], result, scope);
            result = result.withColumn(key, function(row, index){ return computed[index]; });
          });
          return result;
        }
        case 'sortBy': {
          if (node.args.length !== 1) fail(node, 'sortBy expects one expression');
          var keys = this.perRow(node.args[0], frame, scope);
          var descending = node.named.desc ? truthy(this.evaluate(node.named.desc, scope)) : false;
          return frame.sortByKey(function(row, index){ return keys[index]; }, descending);
        }
        case 'agg': {
          var whole = new Grouped(frame, [], [{ key: [], indices: frame.rows().map(function(row, index){ return index; }) }]);
          return whole.agg(this.buildAggSpecs(node, frame, scope));
        }
      }
    }

    switch (name) {
      case 'select': {
        var picked = node.args.length === 1 && Array.isArray(this.evaluate(node.args[0], scope))
          ? this.evaluate(node.args[0], scope)
          : node.args.map(function(argument){ return self.evaluate(argument, scope); });
        return frame.select(this.columnList(frame, picked, node));
      }
      case 'drop': {
        var dropped = node.args.map(function(argument){ return self.evaluate(argument, scope); });
        var flat = dropped.length === 1 && Array.isArray(dropped[0]) ? dropped[0] : dropped;
        return frame.drop(this.columnList(frame, flat, node));
      }
      case 'rename': {
        if (!node.namedOrder.length) fail(node, 'rename expects named columns, for example rename(old = "new")');
        var mapping = {};
        node.namedOrder.forEach(function(key){
          self.expectColumn(frame, key, node);
          mapping[key] = String(self.evaluate(node.named[key], scope));
        });
        return frame.rename(mapping);
      }
      case 'sort': {
        var options = this.readArguments(node, scope, [
          { name: 'by', required: true },
          { name: 'desc', fallback: false }
        ]);
        return frame.sort(this.columnList(frame, options.by, node), truthy(options.desc));
      }
      case 'head':
      case 'limit': {
        var headOptions = this.readArguments(node, scope, [{ name: 'n', fallback: 10 }]);
        return frame.head(Math.trunc(toNumber(headOptions.n, node, 'n')));
      }
      case 'tail': {
        var tailOptions = this.readArguments(node, scope, [{ name: 'n', fallback: 10 }]);
        return frame.tail(Math.trunc(toNumber(tailOptions.n, node, 'n')));
      }
      case 'slice': {
        var sliceOptions = this.readArguments(node, scope, [
          { name: 'start', fallback: 0 },
          { name: 'end', fallback: null }
        ]);
        return frame.slice(
          Math.trunc(toNumber(sliceOptions.start, node, 'start')),
          sliceOptions.end === null ? null : Math.trunc(toNumber(sliceOptions.end, node, 'end'))
        );
      }
      case 'groupby': {
        var groupArgs = node.args.map(function(argument){ return self.evaluate(argument, scope); });
        var groupKeys = groupArgs.length === 1 && Array.isArray(groupArgs[0]) ? groupArgs[0] : groupArgs;
        if (!groupKeys.length) fail(node, 'groupby expects at least one column');
        return frame.groupby(this.columnList(frame, groupKeys, node));
      }
      case 'pivot': {
        var pivotOptions = this.readArguments(node, scope, [
          { name: 'index', required: true },
          { name: 'columns', required: true },
          { name: 'values', required: true },
          { name: 'agg', fallback: 'sum' }
        ]);
        return frame.pivot(
          this.expectColumn(frame, pivotOptions.index, node),
          this.expectColumn(frame, pivotOptions.columns, node),
          this.expectColumn(frame, pivotOptions.values, node),
          this.aggName(pivotOptions.agg, node)
        );
      }
      case 'join': {
        var joinOptions = this.readArguments(node, scope, [
          { name: 'other', required: true },
          { name: 'on', required: true },
          { name: 'how', fallback: 'inner' }
        ]);
        var other = this.expectFrame(joinOptions.other, node, 'the joined value');
        return frame.join(other, this.columnList(frame, joinOptions.on, node), String(joinOptions.how));
      }
      case 'describe':
        return frame.describe();
      case 'distinct': {
        var distinctArgs = node.args.map(function(argument){ return self.evaluate(argument, scope); });
        var distinctKeys = distinctArgs.length === 1 && Array.isArray(distinctArgs[0]) ? distinctArgs[0] : distinctArgs;
        return frame.distinct(distinctKeys.length ? this.columnList(frame, distinctKeys, node) : null);
      }
      case 'dropna': {
        var naArgs = node.args.map(function(argument){ return self.evaluate(argument, scope); });
        var naKeys = naArgs.length === 1 && Array.isArray(naArgs[0]) ? naArgs[0] : naArgs;
        return frame.dropna(naKeys.length ? this.columnList(frame, naKeys, node) : null);
      }
      case 'fillna': {
        var fillOptions = this.readArguments(node, scope, [
          { name: 'value', required: true },
          { name: 'columns', fallback: null }
        ]);
        return frame.fillna(
          fillOptions.value,
          fillOptions.columns === null ? null : this.columnList(frame, fillOptions.columns, node)
        );
      }
      case 'valueCounts': {
        var countOptions = this.readArguments(node, scope, [
          { name: 'column', required: true },
          { name: 'desc', fallback: true }
        ]);
        return frame.valueCounts(this.expectColumn(frame, countOptions.column, node), truthy(countOptions.desc));
      }
      case 'unique': {
        var uniqueOptions = this.readArguments(node, scope, [{ name: 'column', required: true }]);
        return frame.unique(this.expectColumn(frame, uniqueOptions.column, node));
      }
      case 'concat': {
        var concatOptions = this.readArguments(node, scope, [{ name: 'other', required: true }]);
        return frame.concat(this.expectFrame(concatOptions.other, node, 'the appended value'));
      }
      case 'histogram': {
        var histOptions = this.readArguments(node, scope, [
          { name: 'column', required: true },
          { name: 'bins', fallback: 12 }
        ]);
        return frame.histogram(
          this.expectColumn(frame, histOptions.column, node),
          Math.trunc(toNumber(histOptions.bins, node, 'bins'))
        );
      }
      case 'col':
      case 'column': {
        var colOptions = this.readArguments(node, scope, [{ name: 'name', required: true }]);
        return frame.col(this.expectColumn(frame, colOptions.name, node));
      }
      case 'rows': {
        var rowsOptions = this.readArguments(node, scope, [{ name: 'n', fallback: null }]);
        return frame.rows(rowsOptions.n === null ? null : Math.trunc(toNumber(rowsOptions.n, node, 'n')));
      }
      case 'toCsv':
        return frame.toCsv();
      case 'count':
        return frame.length;
      case 'columns':
        return frame.columns.slice();
      case 'shape':
        return [frame.length, frame.columns.length];
    }

    if (AGG_ALIASES[name]) {
      var aggOptions = this.readArguments(node, scope, [
        { name: 'column', required: true },
        { name: 'ratio', fallback: 0.5 }
      ]);
      return frame.aggregate(
        this.expectColumn(frame, aggOptions.column, node),
        AGG_ALIASES[name] === 'count' ? 'countValid' : AGG_ALIASES[name],
        toNumber(aggOptions.ratio, node, 'ratio')
      );
    }

    fail(node, 'a table has no method "' + name + '"',
      'available: filter, select, drop, rename, sort, head, tail, groupby, agg, pivot, join, describe, withColumn, assign, distinct, dropna, fillna, valueCounts, unique, histogram, count');
    return null;
  };

  Interpreter.prototype.aggName = function(value, node) {
    var name = AGG_ALIASES[String(value)];
    if (!name) {
      fail(node, 'unknown aggregation "' + value + '"', 'available: ' + Object.keys(AGG_ALIASES).join(', '));
    }
    return name === 'count' ? 'countValid' : name;
  };

  Interpreter.prototype.listMethod = function(node, list, scope) {
    var self = this;
    var name = node.name;
    var values = node.args.map(function(argument){ return self.evaluate(argument, scope); });

    if (name === 'count' || name === 'len') return list.length;
    if (name === 'sum') return Frames.aggregations.sum(list);
    if (name === 'mean' || name === 'avg') return Frames.aggregations.mean(list);
    if (name === 'min') return Frames.aggregations.min(list);
    if (name === 'max') return Frames.aggregations.max(list);
    if (name === 'median') return Frames.aggregations.median(list);
    if (name === 'std') return Frames.aggregations.std(list);
    if (name === 'unique') return list.filter(function(value, index){ return list.indexOf(value) === index; });
    if (name === 'sort') {
      var descending = values.length ? truthy(values[0]) : false;
      return list.slice().sort(function(a, b){
        var result = Frames.compareValues(a, b);
        return descending ? -result : result;
      });
    }
    if (name === 'join') return list.map(toText).join(values.length ? String(values[0]) : ', ');
    if (name === 'head') return list.slice(0, values.length ? Math.trunc(toNumber(values[0], node, 'n')) : 10);
    if (name === 'contains') return list.some(function(item){ return equals(item, values[0]); });

    fail(node, 'a list has no method "' + name + '"',
      'available: count, sum, mean, min, max, median, std, unique, sort, join, head, contains');
    return null;
  };

  Interpreter.prototype.textMethod = function(node, text, scope) {
    var self = this;
    var name = node.name;
    var values = node.args.map(function(argument){ return self.evaluate(argument, scope); });

    if (name === 'upper') return text.toUpperCase();
    if (name === 'lower') return text.toLowerCase();
    if (name === 'trim') return text.trim();
    if (name === 'len' || name === 'count') return text.length;
    if (name === 'contains') return text.indexOf(toText(values[0])) !== -1;
    if (name === 'startsWith') return text.indexOf(toText(values[0])) === 0;
    if (name === 'endsWith') {
      var suffix = toText(values[0]);
      return text.length >= suffix.length && text.slice(text.length - suffix.length) === suffix;
    }
    if (name === 'replace') return text.split(toText(values[0])).join(toText(values[1]));
    if (name === 'split') return text.split(toText(values.length ? values[0] : ','));
    if (name === 'slice') {
      return text.slice(
        Math.trunc(toNumber(values[0], node, 'start')),
        values.length > 1 ? Math.trunc(toNumber(values[1], node, 'end')) : undefined
      );
    }

    fail(node, 'text has no method "' + name + '"',
      'available: upper, lower, trim, len, contains, startsWith, endsWith, replace, split, slice');
    return null;
  };

  /* ----------------------------------------------------------- Builtins */

  var BUILTINS = {
    load: true, datasets: true, readCsv: true, table: true, frame: true, save: true,
    print: true, show: true, md: true, note: true,
    bar: true, line: true, area: true, scatter: true, scatter3d: true, pie: true, hist: true, image: true,
    abs: true, round: true, floor: true, ceil: true, sqrt: true, exp: true, log: true,
    min: true, max: true, sum: true, mean: true, median: true, std: true, count: true,
    len: true, num: true, str: true, bool: true, isNull: true, coalesce: true, ifElse: true,
    upper: true, lower: true, trim: true, contains: true, replace: true, split: true,
    year: true, month: true, day: true, weekday: true, dateOf: true, today: true,
    range: true, seq: true, col: true
  };

  var CHART_SIGNATURE = [
    { name: 'data', required: true },
    { name: 'x', fallback: null },
    { name: 'y', fallback: null },
    { name: 'series', fallback: null },
    { name: 'title', fallback: '' },
    { name: 'stacked', fallback: false },
    { name: 'horizontal', fallback: false },
    { name: 'color', fallback: null }
  ];

  Interpreter.prototype.callBuiltin = function(node, scope) {
    var self = this;
    var name = node.callee;

    switch (name) {
      /* ---------------------------------------------------------- data */
      case 'load': {
        var loadOptions = this.readArguments(node, scope, [{ name: 'name', required: true }]);
        var key = String(loadOptions.name);
        var dataset = this.datasets[key];
        if (!dataset) {
          fail(node, 'no dataset named "' + key + '"',
            Object.keys(this.datasets).length
              ? 'available: ' + Object.keys(this.datasets).join(', ')
              : 'import a file first, or call datasets() to list them');
        }
        return dataset.frame;
      }
      case 'datasets': {
        var names = Object.keys(this.datasets);
        return Frames.fromRows(names.map(function(key){
          var dataset = self.datasets[key];
          return {
            name: key,
            rows: dataset.frame.length,
            columns: dataset.frame.columns.length,
            source: dataset.source || 'memory'
          };
        }), ['name', 'rows', 'columns', 'source']);
      }
      case 'readCsv': {
        var csvOptions = this.readArguments(node, scope, [
          { name: 'text', required: true },
          { name: 'delimiter', fallback: null },
          { name: 'header', fallback: null }
        ]);
        try {
          return Csv.parse(String(csvOptions.text), {
            delimiter: csvOptions.delimiter === null ? undefined : String(csvOptions.delimiter),
            header: csvOptions.header === null ? undefined : truthy(csvOptions.header)
          });
        } catch (error) {
          fail(node, 'cannot read this delimited text: ' + error.message);
        }
        return null;
      }
      case 'frame': {
        var frameOptions = this.readArguments(node, scope, [{ name: 'rows', required: true }]);
        if (!Array.isArray(frameOptions.rows)) fail(node, 'frame expects a list of rows');
        try {
          return Frames.fromRows(frameOptions.rows);
        } catch (error) {
          fail(node, error.message);
        }
        return null;
      }
      case 'save': {
        var saveOptions = this.readArguments(node, scope, [
          { name: 'name', required: true },
          { name: 'data', required: true }
        ]);
        var saveName = String(saveOptions.name);
        var saved = this.expectFrame(saveOptions.data, node, 'the saved value');
        this.datasets[saveName] = { frame: saved, source: 'cell' };
        this.emit({ kind: 'stream', stream: 'out', text: 'saved "' + saveName + '" (' + saved.length + ' rows)' });
        return saved;
      }

      /* -------------------------------------------------------- output */
      case 'print': {
        var parts = node.args.map(function(argument){ return toText(self.evaluate(argument, scope)); });
        this.emit({ kind: 'stream', stream: 'out', text: parts.join(' ') });
        return undefined;
      }
      case 'md':
      case 'note': {
        var mdOptions = this.readArguments(node, scope, [{ name: 'text', required: true }]);
        this.emit({ kind: 'markdown', text: toText(mdOptions.text) });
        return undefined;
      }
      case 'show':
      case 'table': {
        var showOptions = this.readArguments(node, scope, [
          { name: 'data', required: true },
          { name: 'limit', fallback: 50 },
          { name: 'title', fallback: '' }
        ]);
        this.emitValue(showOptions.data, {
          limit: Math.trunc(toNumber(showOptions.limit, node, 'limit')),
          title: toText(showOptions.title)
        });
        return undefined;
      }
      case 'image': {
        var imageOptions = this.readArguments(node, scope, [
          { name: 'source', required: true },
          { name: 'title', fallback: '' }
        ]);
        this.emit({ kind: 'image', source: String(imageOptions.source), title: toText(imageOptions.title) });
        return undefined;
      }

      /* -------------------------------------------------------- charts */
      case 'bar':
      case 'line':
      case 'area':
      case 'scatter':
        return this.emitChart(name, node, scope);
      case 'scatter3d':
        return this.emitScatter3d(node, scope);
      case 'pie': {
        var pieOptions = this.readArguments(node, scope, [
          { name: 'data', required: true },
          { name: 'labels', fallback: null },
          { name: 'values', fallback: null },
          { name: 'title', fallback: '' },
          { name: 'color', fallback: null }
        ]);
        var pieFrame = this.expectFrame(pieOptions.data, node, 'the chart data');
        var labelColumn = pieOptions.labels === null ? pieFrame.columns[0] : this.expectColumn(pieFrame, pieOptions.labels, node);
        var valueColumn = pieOptions.values === null ? firstNumeric(pieFrame, labelColumn, node) : this.expectColumn(pieFrame, pieOptions.values, node);
        this.emit({
          kind: 'chart',
          chart: 'pie',
          title: toText(pieOptions.title),
          colors: colorList(pieOptions.color, node),
          labels: pieFrame.col(labelColumn).map(toText),
          axis: { x: labelColumn, y: valueColumn },
          series: [{ name: valueColumn, values: pieFrame.col(valueColumn).map(function(value){ return toNumberOrNull(value); }) }]
        });
        return undefined;
      }
      case 'hist': {
        var histOptions = this.readArguments(node, scope, [
          { name: 'data', required: true },
          { name: 'column', fallback: null },
          { name: 'bins', fallback: 12 },
          { name: 'title', fallback: '' },
          { name: 'color', fallback: null }
        ]);
        var histFrame = this.expectFrame(histOptions.data, node, 'the chart data');
        var histColumn = histOptions.column === null
          ? firstNumeric(histFrame, null, node)
          : this.expectColumn(histFrame, histOptions.column, node);
        var binned = histFrame.histogram(histColumn, Math.trunc(toNumber(histOptions.bins, node, 'bins')));
        this.emit({
          kind: 'chart',
          chart: 'bar',
          title: toText(histOptions.title) || histColumn,
          colors: colorList(histOptions.color, node),
          labels: binned.col('bin').map(function(value){ return toText(Math.round(value * 1000) / 1000); }),
          axis: { x: histColumn, y: 'count' },
          series: [{ name: 'count', values: binned.col('count') }]
        });
        return undefined;
      }

      /* ------------------------------------------------------- scalars */
      case 'col': {
        var colOptions = this.readArguments(node, scope, [{ name: 'name', required: true }]);
        var lookup = scope.lookup(String(colOptions.name));
        if (!lookup.found) {
          fail(node, 'col("' + colOptions.name + '") is only available inside filter, assign, withColumn or agg');
        }
        return lookup.value;
      }
      case 'ifElse': {
        var branchOptions = this.readArguments(node, scope, [
          { name: 'condition', required: true },
          { name: 'then', required: true },
          { name: 'otherwise', fallback: null }
        ]);
        return truthy(branchOptions.condition) ? branchOptions.then : branchOptions.otherwise;
      }
      case 'coalesce': {
        for (var c = 0; c < node.args.length; c += 1) {
          var candidate = this.evaluate(node.args[c], scope);
          if (candidate !== null && candidate !== undefined && !(typeof candidate === 'number' && isNaN(candidate))) {
            return candidate;
          }
        }
        return null;
      }
      case 'isNull': {
        var nullOptions = this.readArguments(node, scope, [{ name: 'value', required: true }]);
        return Frames.isMissing(nullOptions.value);
      }
    }

    var values = node.args.map(function(argument){ return self.evaluate(argument, scope); });
    var flat = values.length === 1 && Array.isArray(values[0]) ? values[0] : values;

    switch (name) {
      case 'abs': return nullable(values[0], function(v){ return Math.abs(toNumber(v, node)); });
      case 'floor': return nullable(values[0], function(v){ return Math.floor(toNumber(v, node)); });
      case 'ceil': return nullable(values[0], function(v){ return Math.ceil(toNumber(v, node)); });
      case 'sqrt': return nullable(values[0], function(v){ return Math.sqrt(toNumber(v, node)); });
      case 'exp': return nullable(values[0], function(v){ return Math.exp(toNumber(v, node)); });
      case 'log': return nullable(values[0], function(v){ var x = toNumber(v, node); return x > 0 ? Math.log(x) : null; });
      case 'round': {
        var digits = values.length > 1 ? Math.trunc(toNumber(values[1], node, 'digits')) : 0;
        var factor = Math.pow(10, digits);
        return nullable(values[0], function(v){ return Math.round(toNumber(v, node) * factor) / factor; });
      }
      case 'min': return Frames.aggregations.min(flat);
      case 'max': return Frames.aggregations.max(flat);
      case 'sum': return Frames.aggregations.sum(flat);
      case 'mean': return Frames.aggregations.mean(flat);
      case 'median': return Frames.aggregations.median(flat);
      case 'std': return Frames.aggregations.std(flat);
      case 'count': return flat.length;
      case 'len': {
        var subject = values[0];
        if (typeof subject === 'string') return subject.length;
        if (Array.isArray(subject)) return subject.length;
        if (isFrame(subject)) return subject.length;
        return 0;
      }
      case 'num': return values[0] === null || values[0] === undefined ? null : toNumber(values[0], node);
      case 'str': return toText(values[0]);
      case 'bool': return truthy(values[0]);
      case 'upper': return toText(values[0]).toUpperCase();
      case 'lower': return toText(values[0]).toLowerCase();
      case 'trim': return toText(values[0]).trim();
      case 'contains': return toText(values[0]).indexOf(toText(values[1])) !== -1;
      case 'replace': return toText(values[0]).split(toText(values[1])).join(toText(values[2]));
      case 'split': return toText(values[0]).split(toText(values.length > 1 ? values[1] : ','));
      case 'dateOf': return asDate(values[0]);
      case 'today': return new Date();
      case 'year': return datePart(asDate(values[0]), 'year');
      case 'month': return datePart(asDate(values[0]), 'month');
      case 'day': return datePart(asDate(values[0]), 'day');
      case 'weekday': return datePart(asDate(values[0]), 'weekday');
      case 'range':
      case 'seq': {
        var start = values.length > 1 ? toNumber(values[0], node, 'start') : 0;
        var stop = values.length > 1 ? toNumber(values[1], node, 'stop') : toNumber(values[0], node, 'stop');
        var step = values.length > 2 ? toNumber(values[2], node, 'step') : 1;
        if (step === 0) fail(node, 'range step cannot be zero');
        var out = [];
        for (var value = start; step > 0 ? value < stop : value > stop; value += step) {
          out.push(value);
          if (out.length % 1024 === 0) this.checkBudget(node);
          if (out.length > this.maxRows) {
            fail(node, 'range would produce more than ' + this.maxRows + ' values');
          }
        }
        return out;
      }
    }

    fail(node, '"' + name + '" is not a known function', this.nameHint(name, scope));
    return null;
  };

  /* Two-part map key that cannot collide: the first part carries its length. */
  function pairKey(left, right) {
    return String(left).length + ":" + left + right;
  }

  /* An explicit colour overrides the validated categorical order. Only plain
   * hex is accepted, so nothing arbitrary reaches the SVG. */
  var HEX = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

  function colorList(value, node) {
    if (value === null || value === undefined) return null;
    var entries = Array.isArray(value) ? value : [value];
    return entries.map(function(entry){
      var text = String(entry).trim();
      if (!HEX.test(text)) {
        fail(node, 'color must be a hex value such as "#ccff00", got "' + text + '"');
      }
      return text;
    });
  }

  function nullable(value, compute) {
    return Frames.isMissing(value) ? null : compute(value);
  }

  function toNumberOrNull(value) {
    if (typeof value === 'number') return isFinite(value) ? value : null;
    if (Frames.isMissing(value)) return null;
    var parsed = Number(value);
    return isNaN(parsed) ? null : parsed;
  }

  function asDate(value) {
    if (value instanceof Date) return value;
    if (Frames.isMissing(value)) return null;
    if (typeof value === 'number') return new Date(value);
    var parsed = Csv.parseDate(String(value));
    return parsed || null;
  }

  function datePart(value, part) {
    if (!value) return null;
    if (part === 'year') return value.getUTCFullYear();
    if (part === 'month') return value.getUTCMonth() + 1;
    if (part === 'day') return value.getUTCDate();
    return value.getUTCDay();
  }

  function firstNumeric(frame, exclude, node) {
    for (var i = 0; i < frame.columns.length; i += 1) {
      var name = frame.columns[i];
      if (name === exclude) continue;
      if (frame.dtype(name) === 'number') return name;
    }
    fail(node, 'no numeric column found in this table', 'columns: ' + frame.columns.join(', '));
    return null;
  }

  Interpreter.prototype.emitChart = function(kind, node, scope) {
    var self = this;
    var options = this.readArguments(node, scope, CHART_SIGNATURE);
    var frame = this.expectFrame(options.data, node, 'the chart data');
    if (!frame.length) fail(node, 'the chart data is empty');

    var xColumn = options.x === null ? frame.columns[0] : this.expectColumn(frame, options.x, node);

    var yColumns;
    if (options.y === null) {
      yColumns = frame.columns.filter(function(name){
        return name !== xColumn && frame.dtype(name) === 'number';
      });
      if (!yColumns.length) fail(node, 'no numeric column to plot', 'columns: ' + frame.columns.join(', '));
    } else {
      yColumns = this.columnList(frame, options.y, node);
    }

    var labels = frame.col(xColumn).map(toText);
    var series;

    if (options.series !== null) {
      var seriesColumn = this.expectColumn(frame, String(options.series), node);
      var valueColumn = yColumns[0];
      var categories = frame.unique(seriesColumn).map(toText);
      var uniqueLabels = [];
      var seen = Object.create(null);
      labels.forEach(function(label){ if (!seen[label]) { seen[label] = true; uniqueLabels.push(label); } });
      var lookup = Object.create(null);
      for (var i = 0; i < frame.length; i += 1) {
        lookup[pairKey(toText(frame.data[xColumn][i]), toText(frame.data[seriesColumn][i]))] =
          toNumberOrNull(frame.data[valueColumn][i]);
      }
      labels = uniqueLabels;
      series = categories.map(function(category){
        return {
          name: category,
          values: uniqueLabels.map(function(label){
            var value = lookup[pairKey(label, category)];
            return value === undefined ? null : value;
          })
        };
      });
    } else {
      series = yColumns.map(function(name){
        return { name: name, values: frame.col(name).map(toNumberOrNull) };
      });
    }

    this.emit({
      kind: 'chart',
      chart: kind,
      title: toText(options.title),
      labels: labels,
      axis: { x: xColumn, y: yColumns.join(', ') },
      stacked: truthy(options.stacked),
      horizontal: truthy(options.horizontal),
      colors: colorList(options.color, node),
      series: series
    });
    return undefined;
  };

  /* Three measures plotted on three axes. Each point carries its own values,
   * so the renderer can sort by depth and the table view can list them. */
  Interpreter.prototype.emitScatter3d = function(node, scope) {
    var options = this.readArguments(node, scope, [
      { name: 'data', required: true },
      { name: 'x', required: true },
      { name: 'y', required: true },
      { name: 'z', required: true },
      { name: 'series', fallback: null },
      { name: 'size', fallback: null },
      { name: 'label', fallback: null },
      { name: 'title', fallback: '' },
      { name: 'color', fallback: null }
    ]);

    var frame = this.expectFrame(options.data, node, 'the chart data');
    if (!frame.length) fail(node, 'the chart data is empty');

    var xColumn = this.expectColumn(frame, options.x, node);
    var yColumn = this.expectColumn(frame, options.y, node);
    var zColumn = this.expectColumn(frame, options.z, node);
    [xColumn, yColumn, zColumn].forEach(function(name){
      if (frame.dtype(name) !== 'number') {
        fail(node, 'column "' + name + '" is ' + frame.dtype(name) + ', a numeric column is required for an axis');
      }
    });

    var seriesColumn = options.series === null ? null : this.expectColumn(frame, options.series, node);
    var sizeColumn = options.size === null ? null : this.expectColumn(frame, options.size, node);
    var labelColumn = options.label === null ? null : this.expectColumn(frame, options.label, node);

    var points = [];
    for (var i = 0; i < frame.length; i += 1) {
      var px = frame.data[xColumn][i];
      var py = frame.data[yColumn][i];
      var pz = frame.data[zColumn][i];
      if (!Frames.isNumeric(px) || !Frames.isNumeric(py) || !Frames.isNumeric(pz)) continue;
      points.push({
        x: px,
        y: py,
        z: pz,
        series: seriesColumn ? toText(frame.data[seriesColumn][i]) : null,
        size: sizeColumn ? toNumberOrNull(frame.data[sizeColumn][i]) : null,
        label: labelColumn ? toText(frame.data[labelColumn][i]) : null
      });
    }

    if (!points.length) fail(node, 'no row has all three coordinates');

    this.emit({
      kind: 'chart',
      chart: 'scatter3d',
      title: toText(options.title),
      axis: { x: xColumn, y: yColumn, z: zColumn, series: seriesColumn },
      colors: colorList(options.color, node),
      points: points
    });
    return undefined;
  };

  Interpreter.prototype.emitValue = function(value, options) {
    var settings = options || {};
    if (isFrame(value)) {
      var limit = settings.limit == null ? 50 : settings.limit;
      this.emit({
        kind: 'table',
        title: settings.title || '',
        columns: value.columns.slice(),
        schema: value.schema(),
        rows: value.rows(limit).map(function(row){
          return value.columns.map(function(name){ return serialize(row[name]); });
        }),
        total: value.length,
        truncated: value.length > limit
      });
      return;
    }
    if (isGrouped(value)) {
      this.emit({ kind: 'stream', stream: 'out', text: 'grouped table on ' + value.keys.join(', ') + ' (' + value.groups.length + ' groups)' });
      return;
    }
    if (Array.isArray(value)) {
      this.emit({ kind: 'value', text: toText(value), valueType: 'list', count: value.length });
      return;
    }
    this.emit({ kind: 'value', text: toText(value), valueType: typeName(value) });
  };

  function serialize(value) {
    if (value instanceof Date) return { __date: value.toISOString() };
    if (typeof value === 'number' && !isFinite(value)) return null;
    return value === undefined ? null : value;
  }

  /* ------------------------------------------------------------- Facade */

  /* Runs one cell. Returns collected outputs and, on failure, a structured
   * error carrying the line and column so the editor can point at it. */
  function execute(source, options) {
    var settings = options || {};
    var outputs = [];
    var interpreter = new Interpreter({
      datasets: settings.datasets,
      deadline: settings.timeoutMs ? Date.now() + settings.timeoutMs : 0,
      token: settings.token,
      emit: function(output){
        outputs.push(output);
        if (settings.onOutput) settings.onOutput(output);
      }
    });

    if (settings.variables) {
      Object.keys(settings.variables).forEach(function(name){
        interpreter.scope.set(name, settings.variables[name]);
      });
    }

    try {
      var program = parse(source);
      var last = interpreter.run(program);
      if (last.has && last.value !== null && last.value !== undefined) {
        interpreter.emitValue(last.value, { limit: 50 });
      }
      return { ok: true, outputs: outputs, scope: interpreter.scope.variables, datasets: interpreter.datasets };
    } catch (error) {
      return {
        ok: false,
        outputs: outputs,
        scope: interpreter.scope.variables,
        datasets: interpreter.datasets,
        error: {
          message: error.message || String(error),
          line: error.line || 0,
          column: error.column || 0,
          hint: error.hint || '',
          cancelled: !!error.cancelled,
          timeout: !!error.timeout
        }
      };
    }
  }

  function describeVariables(variables) {
    return Object.keys(variables).map(function(name){
      var value = variables[name];
      var summary;
      if (isFrame(value)) {
        summary = value.length + ' rows x ' + value.columns.length + ' columns';
      } else if (isGrouped(value)) {
        summary = value.groups.length + ' groups';
      } else if (Array.isArray(value)) {
        summary = value.length + ' items';
      } else {
        summary = toText(value).slice(0, 80);
      }
      return { name: name, type: typeName(value), summary: summary };
    });
  }

  return Object.freeze({
    tokenize: tokenize,
    parse: parse,
    execute: execute,
    describeVariables: describeVariables,
    typeName: typeName,
    toText: toText,
    truthy: truthy,
    builtins: Object.keys(BUILTINS),
    error: StudioError
  });
});
