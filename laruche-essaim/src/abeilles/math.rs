use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;

/// Evaluate a mathematical expression safely.
pub struct MathEval;

#[async_trait]
impl Abeille for MathEval {
    fn nom(&self) -> &str {
        "math_eval"
    }

    fn description(&self) -> &str {
        "Evaluate a mathematical expression. Supports: +, -, *, /, %, ** (power), \
         parentheses, and common functions like sqrt, abs, sin, cos, pi, e. \
         Use this for precise calculations instead of computing in your head."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "The mathematical expression to evaluate (e.g., '(42 * 3.14) + sqrt(16)')"
                }
            },
            "required": ["expression"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let expr = args["expression"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'expression' argument"))?;

        match eval_math(expr) {
            Ok(result) => Ok(ResultatAbeille::ok(format!("{} = {}", expr, result))),
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Failed to evaluate '{}': {}",
                expr, e
            ))),
        }
    }
}

/// Simple recursive descent math parser.
/// Supports: +, -, *, /, %, parentheses, unary minus, and functions.
fn eval_math(expr: &str) -> Result<f64, String> {
    let tokens = tokenize(expr)?;
    let mut pos = 0;
    let result = parse_expr(&tokens, &mut pos)?;
    if pos < tokens.len() {
        return Err(format!("Unexpected token: {:?}", tokens[pos]));
    }
    Ok(result)
}

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Op(char),
    Func(String),
    LParen,
    RParen,
}

fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' => {
                chars.next();
            }
            '0'..='9' | '.' => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let n: f64 = num_str.parse().map_err(|_| format!("Invalid number: {}", num_str))?;
                tokens.push(Token::Num(n));
            }
            'a'..='z' | 'A'..='Z' => {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                // Check for constants
                match name.to_lowercase().as_str() {
                    "pi" => tokens.push(Token::Num(std::f64::consts::PI)),
                    "e" => tokens.push(Token::Num(std::f64::consts::E)),
                    _ => tokens.push(Token::Func(name.to_lowercase())),
                }
            }
            '+' | '-' | '*' | '/' | '%' | '^' => {
                tokens.push(Token::Op(ch));
                chars.next();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            _ => {
                chars.next(); // Skip unknown chars
            }
        }
    }

    Ok(tokens)
}

fn parse_expr(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_term(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Op('+') => {
                *pos += 1;
                left += parse_term(tokens, pos)?;
            }
            Token::Op('-') => {
                *pos += 1;
                left -= parse_term(tokens, pos)?;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_term(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    let mut left = parse_power(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Op('*') => {
                *pos += 1;
                left *= parse_power(tokens, pos)?;
            }
            Token::Op('/') => {
                *pos += 1;
                let right = parse_power(tokens, pos)?;
                if right == 0.0 {
                    return Err("Division by zero".to_string());
                }
                left /= right;
            }
            Token::Op('%') => {
                *pos += 1;
                let right = parse_power(tokens, pos)?;
                left %= right;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_power(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    let base = parse_unary(tokens, pos)?;
    if *pos < tokens.len() {
        if let Token::Op('^') = &tokens[*pos] {
            *pos += 1;
            let exp = parse_power(tokens, pos)?; // Right-associative
            return Ok(base.powf(exp));
        }
    }
    Ok(base)
}

fn parse_unary(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    if *pos < tokens.len() {
        if let Token::Op('-') = &tokens[*pos] {
            *pos += 1;
            let val = parse_atom(tokens, pos)?;
            return Ok(-val);
        }
        if let Token::Op('+') = &tokens[*pos] {
            *pos += 1;
        }
    }
    parse_atom(tokens, pos)
}

fn parse_atom(tokens: &[Token], pos: &mut usize) -> Result<f64, String> {
    if *pos >= tokens.len() {
        return Err("Unexpected end of expression".to_string());
    }

    match &tokens[*pos] {
        Token::Num(n) => {
            let n = *n;
            *pos += 1;
            Ok(n)
        }
        Token::Func(name) => {
            let name = name.clone();
            *pos += 1;
            // Expect parenthesis
            if *pos < tokens.len() && matches!(&tokens[*pos], Token::LParen) {
                *pos += 1;
                let arg = parse_expr(tokens, pos)?;
                if *pos < tokens.len() && matches!(&tokens[*pos], Token::RParen) {
                    *pos += 1;
                }
                match name.as_str() {
                    "sqrt" => Ok(arg.sqrt()),
                    "abs" => Ok(arg.abs()),
                    "sin" => Ok(arg.sin()),
                    "cos" => Ok(arg.cos()),
                    "tan" => Ok(arg.tan()),
                    "log" | "ln" => Ok(arg.ln()),
                    "log10" => Ok(arg.log10()),
                    "log2" => Ok(arg.log2()),
                    "ceil" => Ok(arg.ceil()),
                    "floor" => Ok(arg.floor()),
                    "round" => Ok(arg.round()),
                    "exp" => Ok(arg.exp()),
                    _ => Err(format!("Unknown function: {}", name)),
                }
            } else {
                Err(format!("Expected '(' after function '{}'", name))
            }
        }
        Token::LParen => {
            *pos += 1;
            let val = parse_expr(tokens, pos)?;
            if *pos < tokens.len() && matches!(&tokens[*pos], Token::RParen) {
                *pos += 1;
            }
            Ok(val)
        }
        other => Err(format!("Unexpected token: {:?}", other)),
    }
}
