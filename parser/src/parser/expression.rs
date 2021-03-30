// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.

use tendril::format_tendril;

use super::*;

const INT_TYPES: &[Token] = &[
    Token::I8,
    Token::I16,
    Token::I32,
    Token::I64,
    Token::I128,
    Token::U8,
    Token::U16,
    Token::U32,
    Token::U64,
    Token::U128,
    Token::Field,
    Token::Group,
];

impl ParserContext {
    ///
    /// Returns an [`Expression`] AST node if the next token is an expression.
    /// Includes circuit init expressions.
    ///
    pub fn parse_expression(&mut self) -> SyntaxResult<Expression> {
        // Store current parser state.
        let prior_fuzzy_state = self.fuzzy_struct_state;

        // Allow circuit init expressions.
        self.fuzzy_struct_state = false;

        // Parse expression.
        let result = self.parse_expression_fuzzy();

        // Restore prior parser state.
        self.fuzzy_struct_state = prior_fuzzy_state;

        result
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent
    /// a ternary expression. May or may not include circuit init expressions.
    ///
    /// Otherwise, tries to parse the next token using [`parse_or_expression`].
    ///
    pub fn parse_expression_fuzzy(&mut self) -> SyntaxResult<Expression> {
        // Try to parse the next expression. Try BinaryOperation::Or.
        let mut expr = self.parse_or_expression()?;

        // Parse the rest of the ternary expression.
        if self.eat(Token::Question).is_some() {
            let if_true = self.parse_expression()?;
            self.expect(Token::Colon)?;
            let if_false = self.parse_expression_fuzzy()?;
            expr = Expression::Ternary(TernaryExpression {
                span: expr.span() + if_false.span(),
                condition: Box::new(expr),
                if_true: Box::new(if_true),
                if_false: Box::new(if_false),
            });
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent
    /// a binary or expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_and_expression`].
    ///
    pub fn parse_or_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_and_expression()?;
        while self.eat(Token::Or).is_some() {
            let right = self.parse_and_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: BinaryOperation::Or,
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary and expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_bit_or_expression`].
    ///
    pub fn parse_and_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_bit_or_expression()?;
        while self.eat(Token::And).is_some() {
            let right = self.parse_bit_or_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: BinaryOperation::And,
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary bitwise or expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_bit_xor_expression`].
    ///
    pub fn parse_bit_or_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_bit_xor_expression()?;
        while self.eat(Token::BitOr).is_some() {
            let right = self.parse_bit_xor_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: BinaryOperation::BitOr,
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary bitwise xor expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_bit_and_expression`].
    ///
    pub fn parse_bit_xor_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_bit_and_expression()?;
        while self.eat(Token::BitXor).is_some() {
            let right = self.parse_bit_and_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: BinaryOperation::BitXor,
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary bitwise and expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_eq_expression`].
    ///
    pub fn parse_bit_and_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_eq_expression()?;
        while self.eat(Token::BitAnd).is_some() {
            let right = self.parse_eq_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: BinaryOperation::BitAnd,
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary equals or not equals expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_rel_expression`].
    ///
    pub fn parse_eq_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_rel_expression()?;
        while let Some(SpannedToken { token: op, .. }) = self.eat_any(&[Token::Eq, Token::NotEq]) {
            let right = self.parse_rel_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: match op {
                    Token::Eq => BinaryOperation::Eq,
                    Token::NotEq => BinaryOperation::Ne,
                    _ => unimplemented!(),
                },
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary relational expression: less than, less than or equals, greater than, greater than or equals.
    ///
    /// Otherwise, tries to parse the next token using [`parse_shift_expression`].
    ///    
    pub fn parse_rel_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_shift_expression()?;
        while let Some(SpannedToken { token: op, .. }) = self.eat_any(&[Token::Lt, Token::LtEq, Token::Gt, Token::GtEq])
        {
            let right = self.parse_shift_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: match op {
                    Token::Lt => BinaryOperation::Lt,
                    Token::LtEq => BinaryOperation::Le,
                    Token::Gt => BinaryOperation::Gt,
                    Token::GtEq => BinaryOperation::Ge,
                    _ => unimplemented!(),
                },
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary shift expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_add_sub_expression`].
    ///
    pub fn parse_shift_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_add_sub_expression()?;
        while let Some(SpannedToken { token: op, .. }) = self.eat_any(&[Token::Shl, Token::Shr, Token::ShrSigned]) {
            let right = self.parse_add_sub_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: match op {
                    Token::Shl => BinaryOperation::Shl,
                    Token::Shr => BinaryOperation::Shr,
                    Token::ShrSigned => BinaryOperation::ShrSigned,
                    _ => unimplemented!(),
                },
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary addition or subtraction expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_mul_div_pow_expression`].
    ///
    pub fn parse_add_sub_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_mul_div_mod_expression()?;
        while let Some(SpannedToken { token: op, .. }) = self.eat_any(&[Token::Add, Token::Minus]) {
            let right = self.parse_mul_div_mod_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: match op {
                    Token::Add => BinaryOperation::Add,
                    Token::Minus => BinaryOperation::Sub,
                    _ => unimplemented!(),
                },
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary multiplication, division, or modulus expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_exp_expression`].
    ///
    pub fn parse_mul_div_mod_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_exp_expression()?;
        while let Some(SpannedToken { token: op, .. }) = self.eat_any(&[Token::Mul, Token::Div, Token::Mod]) {
            let right = self.parse_exp_expression()?;
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + right.span(),
                op: match op {
                    Token::Mul => BinaryOperation::Mul,
                    Token::Div => BinaryOperation::Div,
                    Token::Mod => BinaryOperation::Mod,
                    _ => unimplemented!(),
                },
                left: Box::new(expr),
                right: Box::new(right),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// binary exponentiation expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_cast_expression`].
    ///
    pub fn parse_exp_expression(&mut self) -> SyntaxResult<Expression> {
        let mut exprs = vec![self.parse_cast_expression()?];

        while self.eat(Token::Exp).is_some() {
            exprs.push(self.parse_cast_expression()?);
        }
        let mut expr = exprs.remove(exprs.len() - 1);
        while !exprs.is_empty() {
            let sub_expr = exprs.remove(exprs.len() - 1);
            expr = Expression::Binary(BinaryExpression {
                span: expr.span() + sub_expr.span(),
                op: BinaryOperation::Pow,
                left: Box::new(sub_expr),
                right: Box::new(expr),
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// type cast expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_unary_expression`].
    ///
    pub fn parse_cast_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_unary_expression()?;
        while self.eat(Token::As).is_some() {
            let (type_, type_span) = self.parse_type()?;
            expr = Expression::Cast(CastExpression {
                span: expr.span() + &type_span,
                inner: Box::new(expr),
                target_type: type_,
            })
        }
        Ok(expr)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent a
    /// unary not, negate, or bitwise not expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_access_expression`].
    ///
    pub fn parse_unary_expression(&mut self) -> SyntaxResult<Expression> {
        let mut ops = Vec::new();
        while let Some(token) = self.eat_any(&[Token::Not, Token::Minus, Token::BitNot]) {
            ops.push(token);
        }
        let mut inner = self.parse_access_expression()?;
        for op in ops.into_iter().rev() {
            let operation = match op.token {
                Token::Not => UnaryOperation::Not,
                Token::Minus => UnaryOperation::Negate,
                Token::BitNot => UnaryOperation::BitNot,
                _ => unimplemented!(),
            };
            // hack for const signed integer overflow issues
            if matches!(operation, UnaryOperation::Negate) {
                if let Expression::Value(ValueExpression::Integer(type_, value, span)) = inner {
                    inner = Expression::Value(ValueExpression::Integer(
                        type_,
                        format_tendril!("-{}", value),
                        &op.span + &span,
                    ));
                    continue;
                } else if let Expression::Value(ValueExpression::Implicit(value, span)) = inner {
                    inner = Expression::Value(ValueExpression::Implicit(
                        format_tendril!("-{}", value),
                        &op.span + &span,
                    ));
                    continue;
                }
            }
            inner = Expression::Unary(UnaryExpression {
                span: &op.span + inner.span(),
                op: operation,
                inner: Box::new(inner),
            });
        }
        Ok(inner)
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent an
    /// array access, circuit member access, function call, or static function call expression.
    ///
    /// Otherwise, tries to parse the next token using [`parse_primary_expression`].
    ///
    pub fn parse_access_expression(&mut self) -> SyntaxResult<Expression> {
        let mut expr = self.parse_primary_expression()?;
        while let Some(token) = self.eat_any(&[Token::LeftSquare, Token::Dot, Token::LeftParen, Token::DoubleColon]) {
            match token.token {
                Token::LeftSquare => {
                    if self.eat(Token::DotDot).is_some() {
                        let right = if self.peek()?.token != Token::RightSquare {
                            Some(Box::new(self.parse_expression()?))
                        } else {
                            None
                        };

                        let end = self.expect(Token::RightSquare)?;
                        expr = Expression::ArrayRangeAccess(ArrayRangeAccessExpression {
                            span: expr.span() + &end,
                            array: Box::new(expr),
                            left: None,
                            right,
                        });
                        continue;
                    }

                    let left = self.parse_expression()?;
                    if self.eat(Token::DotDot).is_some() {
                        let right = if self.peek()?.token != Token::RightSquare {
                            Some(Box::new(self.parse_expression()?))
                        } else {
                            None
                        };

                        let end = self.expect(Token::RightSquare)?;
                        expr = Expression::ArrayRangeAccess(ArrayRangeAccessExpression {
                            span: expr.span() + &end,
                            array: Box::new(expr),
                            left: Some(Box::new(left)),
                            right,
                        });
                    } else {
                        let end = self.expect(Token::RightSquare)?;
                        expr = Expression::ArrayAccess(ArrayAccessExpression {
                            span: expr.span() + &end,
                            array: Box::new(expr),
                            index: Box::new(left),
                        });
                    }
                }
                Token::Dot => {
                    if let Some(ident) = self.eat_identifier() {
                        expr = Expression::CircuitMemberAccess(CircuitMemberAccessExpression {
                            span: expr.span() + &ident.span,
                            circuit: Box::new(expr),
                            name: ident,
                        });
                    } else if let Some((num, span)) = self.eat_int() {
                        expr = Expression::TupleAccess(TupleAccessExpression {
                            span: expr.span() + &span,
                            tuple: Box::new(expr),
                            index: num,
                        });
                    } else {
                        let next = self.peek()?;
                        return Err(SyntaxError::unexpected_str(&next.token, "int or ident", &next.span));
                    }
                }
                Token::LeftParen => {
                    let mut arguments = Vec::new();
                    let end_span;
                    loop {
                        let end = self.eat(Token::RightParen);
                        if let Some(end) = end {
                            end_span = end.span;
                            break;
                        }
                        arguments.push(self.parse_expression()?);
                        if self.eat(Token::Comma).is_none() {
                            end_span = self.expect(Token::RightParen)?;
                            break;
                        }
                    }
                    expr = Expression::Call(CallExpression {
                        span: expr.span() + &end_span,
                        function: Box::new(expr),
                        arguments,
                    });
                }
                Token::DoubleColon => {
                    let ident = self.expect_ident()?;
                    expr = Expression::CircuitStaticFunctionAccess(CircuitStaticFunctionAccessExpression {
                        span: expr.span() + &ident.span,
                        circuit: Box::new(expr),
                        name: ident,
                    });
                }
                _ => unimplemented!(),
            }
        }
        Ok(expr)
    }

    ///
    /// Returns a [`SpreadOrExpression`] AST node if the next tokens represent an
    /// spread or expression.
    ///
    /// This method should only be called in the context of an array access expression.
    ///
    pub fn parse_spread_or_expression(&mut self) -> SyntaxResult<SpreadOrExpression> {
        Ok(if self.eat(Token::DotDotDot).is_some() {
            SpreadOrExpression::Spread(self.parse_expression()?)
        } else {
            SpreadOrExpression::Expression(self.parse_expression()?)
        })
    }

    ///
    /// Returns an [`Expression`] AST node if the next tokens represent an
    /// circuit initialization expression.
    ///
    pub fn parse_circuit_init(&mut self, identifier: Identifier) -> SyntaxResult<Expression> {
        self.expect(Token::LeftCurly)?;
        let mut members = Vec::new();
        let end_span;
        loop {
            if let Some(end) = self.eat(Token::RightCurly) {
                end_span = end.span;
                break;
            }
            let name = self.expect_ident()?;
            if self.eat(Token::Colon).is_some() {
                let expression = self.parse_expression()?;
                members.push(CircuitImpliedVariableDefinition {
                    identifier: name,
                    expression: Some(expression),
                });
            } else {
                members.push(CircuitImpliedVariableDefinition {
                    identifier: name.clone(),
                    expression: None,
                });
            }
            if self.eat(Token::Comma).is_none() {
                end_span = self.expect(Token::RightCurly)?;
                break;
            }
        }
        Ok(Expression::CircuitInit(CircuitInitExpression {
            span: &identifier.span + &end_span,
            name: identifier,
            members,
        }))
    }

    ///
    /// Returns an [`Expression`] AST node if the next token is a primary expression:
    /// - Scalar types: field, group, unsigned integer, signed integer, boolean, address
    /// - Aggregate types: array, tuple
    /// - Identifiers: variables, keywords
    ///
    /// Returns an expression error if the token cannot be matched.
    ///
    pub fn parse_primary_expression(&mut self) -> SyntaxResult<Expression> {
        let SpannedToken { token, span } = self.expect_any()?;
        Ok(match token {
            Token::Int(value) => {
                let type_ = self.eat_any(INT_TYPES);
                match type_ {
                    Some(SpannedToken {
                        token: Token::Field,
                        span: type_span,
                    }) => Expression::Value(ValueExpression::Field(value, span + type_span)),
                    Some(SpannedToken {
                        token: Token::Group,
                        span: type_span,
                    }) => Expression::Value(ValueExpression::Group(Box::new(GroupValue::Single(
                        value,
                        span + type_span,
                    )))),
                    Some(SpannedToken { token, span: type_span }) => Expression::Value(ValueExpression::Integer(
                        Self::token_to_int_type(token).expect("unknown int type token"),
                        value,
                        span + type_span,
                    )),
                    None => Expression::Value(ValueExpression::Implicit(value, span)),
                }
            }
            Token::True => Expression::Value(ValueExpression::Boolean("true".into(), span)),
            Token::False => Expression::Value(ValueExpression::Boolean("false".into(), span)),
            Token::AddressLit(value) => Expression::Value(ValueExpression::Address(value, span)),
            Token::Address => {
                self.expect(Token::LeftParen)?;
                let value = self.expect_any()?;
                let value = if let SpannedToken {
                    token: Token::AddressLit(value),
                    ..
                } = value
                {
                    value
                } else {
                    return Err(SyntaxError::unexpected_str(&value.token, "address", &value.span));
                };

                let end = self.expect(Token::RightParen)?;
                Expression::Value(ValueExpression::Address(value, span + end))
            }
            Token::LeftParen => {
                if let Some((left, right, span)) = self.eat_group_partial() {
                    return Ok(Expression::Value(ValueExpression::Group(Box::new(GroupValue::Tuple(
                        GroupTuple {
                            span,
                            x: left,
                            y: right,
                        },
                    )))));
                }
                let mut args = Vec::new();
                let end_span;
                loop {
                    let end = self.eat(Token::RightParen);
                    if let Some(end) = end {
                        end_span = end.span;
                        break;
                    }
                    let expr = self.parse_expression()?;
                    args.push(expr);
                    if self.eat(Token::Comma).is_none() {
                        end_span = self.expect(Token::RightParen)?;
                        break;
                    }
                }
                if args.len() == 1 {
                    args.remove(0)
                } else {
                    Expression::TupleInit(TupleInitExpression {
                        span: span + end_span,
                        elements: args,
                    })
                }
            }
            Token::LeftSquare => {
                if let Some(end) = self.eat(Token::RightSquare) {
                    return Ok(Expression::ArrayInline(ArrayInlineExpression {
                        elements: Vec::new(),
                        span: span + end.span,
                    }));
                }
                let first = self.parse_spread_or_expression()?;
                if self.eat(Token::Semicolon).is_some() {
                    let dimensions = self.parse_array_dimensions()?;
                    let end = self.expect(Token::RightSquare)?;
                    let first = match first {
                        SpreadOrExpression::Spread(first) => {
                            let span = &span + first.span();
                            return Err(SyntaxError::spread_in_array_init(&span));
                        }
                        SpreadOrExpression::Expression(x) => x,
                    };
                    Expression::ArrayInit(ArrayInitExpression {
                        span: span + end,
                        element: Box::new(first),
                        dimensions,
                    })
                } else {
                    let end_span;
                    let mut elements = vec![first];
                    loop {
                        if let Some(token) = self.eat(Token::RightSquare) {
                            end_span = token.span;
                            break;
                        }
                        if elements.len() == 1 {
                            self.expect(Token::Comma)?;
                        }
                        elements.push(self.parse_spread_or_expression()?);
                        if self.eat(Token::Comma).is_none() {
                            end_span = self.expect(Token::RightSquare)?;
                            break;
                        }
                    }
                    Expression::ArrayInline(ArrayInlineExpression {
                        elements,
                        span: span + end_span,
                    })
                }
            }
            Token::Ident(name) => {
                let ident = Identifier { name, span };
                if !self.fuzzy_struct_state && self.peek()?.token == Token::LeftCurly {
                    self.parse_circuit_init(ident)?
                } else {
                    Expression::Identifier(ident)
                }
            }
            Token::BigSelf => {
                let ident = Identifier {
                    name: token.to_string().into(),
                    span,
                };
                if !self.fuzzy_struct_state && self.peek()?.token == Token::LeftCurly {
                    self.parse_circuit_init(ident)?
                } else {
                    Expression::Identifier(ident)
                }
            }
            Token::Input | Token::LittleSelf => {
                let ident = Identifier {
                    name: token.to_string().into(),
                    span,
                };
                Expression::Identifier(ident)
            }
            token => {
                return Err(SyntaxError::unexpected_str(&token, "expression", &span));
            }
        })
    }
}