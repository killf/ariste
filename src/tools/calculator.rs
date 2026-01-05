use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;

/// Simple calculator tool for basic mathematical operations
pub struct CalculatorTool;

impl ToolImpl for CalculatorTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "expression".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Mathematical expression to evaluate (e.g., '2 + 3', '10 * 5', '100 / 4')"
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "calculator".to_string(),
                description: "Perform basic mathematical calculations (+, -, *, /)".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["expression".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let expression = arguments
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'expression' argument".to_string())?;

        // Parse and evaluate the mathematical expression
        match evaluate_expression(expression) {
            Ok(result) => Ok(result.to_string()),
            Err(e) => Err(format!("Evaluation error: {}", e)),
        }
    }
}

/// Simple expression evaluator for basic math operations
fn evaluate_expression(expr: &str) -> Result<f64, String> {
    // Remove whitespace
    let expr = expr.replace(" ", "");

    if expr.is_empty() {
        return Err("Empty expression".to_string());
    }

    // Simple parser for basic operations
    let tokens = tokenize(&expr)?;
    let result = parse_expression(&tokens)?;

    Ok(result)
}

/// Tokenize the expression into numbers and operators
fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();
    let mut current_number = String::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            '0'..='9' | '.' => {
                current_number.push(ch);
                chars.next();
            }
            '+' | '-' | '*' | '/' => {
                if !current_number.is_empty() {
                    tokens.push(Token::Number(
                        current_number
                            .parse::<f64>()
                            .map_err(|_| format!("Invalid number: {}", current_number))?,
                    ));
                    current_number.clear();
                }
                tokens.push(Token::Operator(ch));
                chars.next();
            }
            _ => {
                return Err(format!("Invalid character: {}", ch));
            }
        }
    }

    if !current_number.is_empty() {
        tokens.push(Token::Number(
            current_number
                .parse::<f64>()
                .map_err(|_| format!("Invalid number: {}", current_number))?,
        ));
    }

    Ok(tokens)
}

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Operator(char),
}

/// Parse and evaluate the expression (handles * and / before + and -)
fn parse_expression(tokens: &[Token]) -> Result<f64, String> {
    let mut tokens = tokens.to_vec();
    let mut index = 0;

    // First pass: handle * and /
    while index < tokens.len() {
        if let Token::Operator('*') = tokens[index] {
            if index == 0 || index + 1 >= tokens.len() {
                return Err("Invalid multiplication".to_string());
            }
            if let Token::Number(left) = tokens[index - 1] {
                if let Token::Number(right) = tokens[index + 1] {
                    tokens[index - 1] = Token::Number(left * right);
                    tokens.remove(index);
                    tokens.remove(index);
                    continue;
                }
            }
        } else if let Token::Operator('/') = tokens[index] {
            if index == 0 || index + 1 >= tokens.len() {
                return Err("Invalid division".to_string());
            }
            if let Token::Number(left) = tokens[index - 1] {
                if let Token::Number(right) = tokens[index + 1] {
                    if right == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    tokens[index - 1] = Token::Number(left / right);
                    tokens.remove(index);
                    tokens.remove(index);
                    continue;
                }
            }
        }
        index += 1;
    }

    // Second pass: handle + and -
    let mut result = match tokens.first() {
        Some(Token::Number(n)) => *n,
        _ => return Err("Invalid expression".to_string()),
    };

    let mut index = 1;
    while index < tokens.len() {
        if let Token::Operator(op) = tokens[index] {
            if index + 1 >= tokens.len() {
                return Err("Invalid expression".to_string());
            }
            if let Token::Number(right) = tokens[index + 1] {
                match op {
                    '+' => result += right,
                    '-' => result -= right,
                    _ => return Err(format!("Unexpected operator: {}", op)),
                }
                index += 2;
            } else {
                return Err("Invalid expression".to_string());
            }
        } else {
            return Err("Invalid expression".to_string());
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculator_addition() {
        let tool = CalculatorTool;
        let args = serde_json::json!({"expression": "2 + 3"});
        assert_eq!(tool.execute(&args).await, Ok("5".to_string()));
    }

    #[tokio::test]
    async fn test_calculator_multiplication() {
        let tool = CalculatorTool;
        let args = serde_json::json!({"expression": "10 * 5"});
        assert_eq!(tool.execute(&args).await, Ok("50".to_string()));
    }

    #[tokio::test]
    async fn test_calculator_division() {
        let tool = CalculatorTool;
        let args = serde_json::json!({"expression": "100 / 4"});
        assert_eq!(tool.execute(&args).await, Ok("25".to_string()));
    }

    #[tokio::test]
    async fn test_calculator_complex() {
        let tool = CalculatorTool;
        let args = serde_json::json!({"expression": "10 * 5 + 3"});
        assert_eq!(tool.execute(&args).await, Ok("53".to_string()));
    }

    #[tokio::test]
    async fn test_calculator_division_by_zero() {
        let tool = CalculatorTool;
        let args = serde_json::json!({"expression": "10 / 0"});
        assert!(tool.execute(&args).await.is_err());
    }
}
