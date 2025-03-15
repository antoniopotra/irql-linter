// Copyright Gary Guo.
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::irql::{IrqlRange, IrqlValue};
use crate::preempt_count::ExpectationRange;
use rustc_ast::tokenstream::{self, TokenTree};
use rustc_ast::{ast, token};
use rustc_data_structures::sync::Lrc;
use rustc_errors::{Diag, ErrorGuaranteed};
use rustc_hir::HirId;
use rustc_middle::ty::TyCtxt;
use rustc_span::symbol::Ident;
use rustc_span::Span;

#[derive(Debug, Clone, Copy, Encodable, Decodable, Default)]
pub struct PreemptionCount {
    pub adjustment: Option<i32>,
    pub expectation: Option<ExpectationRange>,
    pub unchecked: bool,
}

#[derive(Debug)]
pub struct Irql {
    pub on_call_requirement: Option<IrqlRange>,
    pub permanent_requirement: Option<IrqlRange>,
    pub on_return_value: Option<IrqlValue>,
}

#[derive(Debug)]
pub enum KlintAttribute {
    PreemptionCount(PreemptionCount),
    DropPreemptionCount(PreemptionCount),
    ReportPreeptionCount,
    DumpMir,
    Irql(Irql),
}

struct Cursor<'a> {
    eof: TokenTree,
    cursor: tokenstream::RefTokenTreeCursor<'a>,
}

impl<'a> Cursor<'a> {
    fn new(cursor: tokenstream::RefTokenTreeCursor<'a>, end_span: Span) -> Self {
        let eof = TokenTree::Token(
            token::Token {
                kind: token::TokenKind::Eof,
                span: end_span,
            },
            tokenstream::Spacing::Alone,
        );
        Cursor { eof, cursor }
    }

    fn is_eof(&self) -> bool {
        self.cursor.look_ahead(0).is_none()
    }

    fn look_ahead(&self, n: usize) -> &TokenTree {
        self.cursor.look_ahead(n).unwrap_or(&self.eof)
    }

    fn next(&mut self) -> &TokenTree {
        self.cursor.next().unwrap_or(&self.eof)
    }
}

struct AttrParser<'tcx> {
    tcx: TyCtxt<'tcx>,
    hir_id: HirId,
}

impl AttrParser<'_> {
    fn error(
        &self,
        span: Span,
        decorate: impl for<'a, 'b> FnOnce(&'b mut Diag<'a, ()>),
    ) -> Result<!, ErrorGuaranteed> {
        self.tcx
            .node_span_lint(crate::INCORRECT_ATTRIBUTE, self.hir_id, span, |lint| {
                lint.primary_message("incorrect usage of klint attributes");
                decorate(lint);
            });
        Err(self
            .tcx
            .dcx()
            .span_delayed_bug(span, "incorrect usage of klint attributes"))
    }

    fn parse_comma_delimited(
        &self,
        mut cursor: Cursor<'_>,
        mut f: impl for<'a> FnMut(Cursor<'a>) -> Result<Cursor<'a>, ErrorGuaranteed>,
    ) -> Result<(), ErrorGuaranteed> {
        loop {
            if cursor.is_eof() {
                return Ok(());
            }

            cursor = f(cursor)?;

            if cursor.is_eof() {
                return Ok(());
            }

            // Check and skip `,`.
            let comma = cursor.next();
            if !matches!(
                comma,
                TokenTree::Token(
                    token::Token {
                        kind: token::TokenKind::Comma,
                        ..
                    },
                    _
                )
            ) {
                self.error(comma.span(), |diag| {
                    diag.help("`,` expected between property values");
                })?;
            }
        }
    }

    fn parse_eq_delimited<'a>(
        &self,
        mut cursor: Cursor<'a>,
        need_eq: impl FnOnce(Ident) -> Result<bool, ErrorGuaranteed>,
        f: impl FnOnce(Ident, Cursor<'a>) -> Result<Cursor<'a>, ErrorGuaranteed>,
    ) -> Result<Cursor<'a>, ErrorGuaranteed> {
        let prop = cursor.next();
        let invalid_prop = |span| {
            self.error(span, |diag| {
                diag.help("identifier expected");
            })?;
        };

        let TokenTree::Token(token, _) = prop else {
            return invalid_prop(prop.span());
        };
        let Some((name, _)) = token.ident() else {
            return invalid_prop(token.span);
        };

        let need_eq = need_eq(name)?;

        // Check and skip `=`.
        let eq = cursor.look_ahead(0);
        let is_eq = matches!(
            eq,
            TokenTree::Token(
                token::Token {
                    kind: token::TokenKind::Eq,
                    ..
                },
                _
            )
        );
        if need_eq && !is_eq {
            self.error(eq.span(), |diag| {
                diag.help("`=` expected after property name");
            })?;
        }
        if !need_eq && is_eq {
            self.error(eq.span(), |diag| {
                diag.help("unexpected `=` after property name");
            })?;
        }

        if is_eq {
            cursor.next();
        }

        cursor = f(name, cursor)?;

        Ok(cursor)
    }

    fn parse_i32<'a>(&self, mut cursor: Cursor<'a>) -> Result<(i32, Cursor<'a>), ErrorGuaranteed> {
        let expect_int = |span| {
            self.error(span, |diag| {
                diag.help("an integer expected");
            })
        };

        let negative = if matches!(
            cursor.look_ahead(0),
            TokenTree::Token(
                token::Token {
                    kind: token::TokenKind::BinOp(token::BinOpToken::Minus),
                    ..
                },
                _
            )
        ) {
            cursor.next();
            true
        } else {
            false
        };

        let token = cursor.next();
        let TokenTree::Token(
            token::Token {
                kind: token::TokenKind::Literal(lit),
                ..
            },
            _,
        ) = token
        else {
            expect_int(token.span())?
        };
        if lit.kind != token::LitKind::Integer || lit.suffix.is_some() {
            expect_int(token.span())?;
        }
        let Some(v) = lit.symbol.as_str().parse::<i32>().ok() else {
            expect_int(token.span())?;
        };
        let v = if negative { -v } else { v };

        Ok((v, cursor))
    }

    fn parse_expectation_range<'a>(
        &self,
        mut cursor: Cursor<'a>,
    ) -> Result<((u32, Option<u32>), Cursor<'a>), ErrorGuaranteed> {
        let expect_range = |span| {
            self.error(span, |diag| {
                diag.help("a range expected");
            })
        };

        let start_span = cursor.look_ahead(0).span();
        let mut start = 0;
        if !matches!(
            cursor.look_ahead(0),
            TokenTree::Token(
                token::Token {
                    kind: token::TokenKind::DotDot | token::TokenKind::DotDotEq,
                    ..
                },
                _
            )
        ) {
            let token = cursor.next();
            let TokenTree::Token(
                token::Token {
                    kind: token::TokenKind::Literal(lit),
                    ..
                },
                _,
            ) = token
            else {
                expect_range(token.span())?
            };
            if lit.kind != token::LitKind::Integer {
                expect_range(token.span())?;
            }
            let Some(v) = lit.symbol.as_str().parse::<u32>().ok() else {
                expect_range(token.span())?;
            };
            start = v;
        }

        let inclusive = match cursor.look_ahead(0) {
            TokenTree::Token(
                token::Token {
                    kind: token::TokenKind::DotDot,
                    ..
                },
                _,
            ) => Some(false),
            TokenTree::Token(
                token::Token {
                    kind: token::TokenKind::DotDotEq,
                    ..
                },
                _,
            ) => Some(true),
            _ => None,
        };

        let mut end = Some(start + 1);
        if let Some(inclusive) = inclusive {
            cursor.next();

            let skip_hi = matches!(
                cursor.look_ahead(0),
                TokenTree::Token(
                    token::Token {
                        kind: token::TokenKind::Comma | token::TokenKind::Eof,
                        ..
                    },
                    _,
                )
            );

            if skip_hi {
                end = None;
            } else {
                let token = cursor.next();
                let TokenTree::Token(
                    token::Token {
                        kind: token::TokenKind::Literal(lit),
                        ..
                    },
                    _,
                ) = token
                else {
                    expect_range(token.span())?
                };
                if lit.kind != token::LitKind::Integer {
                    expect_range(token.span())?;
                }
                let Some(range) = lit.symbol.as_str().parse::<u32>().ok() else {
                    expect_range(token.span())?;
                };

                end = Some(if inclusive { range + 1 } else { range });
            }
        }

        if end.is_some() && end.unwrap() <= start {
            let end_span = cursor.next().span();

            self.error(start_span.until(end_span), |diag| {
                diag.help("the preemption count expectation range must be non-empty");
            })?;
        }

        Ok(((start, end), cursor))
    }

    fn parse_preempt_count(
        &self,
        attr: &ast::Attribute,
        item: &ast::AttrItem,
    ) -> Result<PreemptionCount, ErrorGuaranteed> {
        let mut adjustment = None;
        let mut expectation = None;
        let mut unchecked = false;

        let ast::AttrArgs::Delimited(ast::DelimArgs {
            dspan: delim_span,
            tokens: tts,
            ..
        }) = &item.args
        else {
            self.error(attr.span, |diag| {
                diag.help("correct usage looks like `#[kint::preempt_count(...)]`");
            })?;
        };

        self.parse_comma_delimited(Cursor::new(tts.trees(), delim_span.close), |cursor| {
            self.parse_eq_delimited(
                cursor,
                |name| {
                    Ok(match name.name {
                        v if v == *crate::symbol::adjust => true,
                        v if v == *crate::symbol::expect => true,
                        v if v == *crate::symbol::unchecked => false,
                        _ => {
                            self.error(name.span, |diag| {
                                diag.help(
                                    "unknown property, expected `adjust`, `expect` or `unchecked`",
                                );
                            })?;
                        }
                    })
                },
                |name, mut cursor| {
                    match name.name {
                        v if v == *crate::symbol::adjust => {
                            let v;
                            (v, cursor) = self.parse_i32(cursor)?;
                            adjustment = Some(v);
                        }
                        v if v == *crate::symbol::expect => {
                            let (lo, hi);
                            ((lo, hi), cursor) = self.parse_expectation_range(cursor)?;
                            expectation = Some(ExpectationRange { lo, hi });
                        }
                        v if v == *crate::symbol::unchecked => {
                            unchecked = true;
                        }
                        _ => unreachable!(),
                    }

                    Ok(cursor)
                },
            )
        })?;

        if adjustment.is_none() && expectation.is_none() {
            self.error(item.args.span().unwrap(), |diag| {
                diag.help("at least one of `adjust` or `expect` property must be specified");
            })?;
        }

        Ok(PreemptionCount {
            adjustment,
            expectation,
            unchecked,
        })
    }

    fn parse_irql_value<'a>(
        &self,
        mut cursor: Cursor<'a>,
    ) -> Result<(IrqlValue, Cursor<'a>), ErrorGuaranteed> {
        let token = cursor.next();
        let TokenTree::Token(
            token::Token {
                kind: token::TokenKind::Literal(lit),
                ..
            },
            _,
        ) = token
        else {
            self.error(token.span(), |diag| {
                diag.help("expected an integer between 0 and 31");
            })?;
        };
        if lit.kind != token::LitKind::Integer || lit.suffix.is_some() {
            self.error(token.span(), |diag| {
                diag.help("expected an integer between 0 and 31");
            })?;
        }
        let Some(value) = lit.symbol.as_str().parse::<u32>().ok() else {
            self.error(token.span(), |diag| {
                diag.help("expected an integer between 0 and 31");
            })?;
        };
        if value > 31 {
            self.error(token.span(), |diag| {
                diag.help("expected an integer between 0 and 31");
            })?;
        }

        Ok((IrqlValue { value }, cursor))
    }

    fn parse_irql_range<'a>(
        &self,
        mut cursor: Cursor<'a>,
    ) -> Result<(IrqlRange, Cursor<'a>), ErrorGuaranteed> {
        let low;
        (low, cursor) = self.parse_irql_value(cursor)?;

        match cursor.look_ahead(0) {
            TokenTree::Token(
                token::Token {
                    kind: token::TokenKind::DotDot,
                    ..
                },
                _,
            ) => {
                cursor.next();

                let high;
                (high, cursor) = self.parse_irql_value(cursor)?;

                if high <= low {
                    self.error(cursor.next().span(), |diag| {
                        diag.help("syntax is not a valid range");
                    })?;
                }

                Ok((
                    IrqlRange {
                        low,
                        high: Some(high),
                    },
                    cursor,
                ))
            }
            _ => Ok((IrqlRange { low, high: None }, cursor)),
        }
    }

    fn parse_irql(
        &self,
        attr: &ast::Attribute,
        item: &ast::AttrItem,
    ) -> Result<Irql, ErrorGuaranteed> {
        let mut on_call_requirement = None;
        let mut permanent_requirement = None;
        let mut on_return_value = None;

        let ast::AttrArgs::Delimited(ast::DelimArgs {
            dspan: delim_span,
            tokens: tts,
            ..
        }) = &item.args
        else {
            self.error(attr.span, |diag| {
                diag.help("correct usage looks like `#[kint::irql(...)]`");
            })?;
        };

        self.parse_comma_delimited(Cursor::new(tts.trees(), delim_span.close), |cursor| {
            self.parse_eq_delimited(
                cursor,
                |name| {
                    Ok(match name.name {
                        v if (v == *crate::symbol::require
                            || v == *crate::symbol::always
                            || v == *crate::symbol::raise) =>
                        {
                            true
                        }
                        _ => {
                            self.error(name.span, |diag| {
                                diag.help(
                                    "unknown property, expected `require`, `always` or `raise`",
                                );
                            })?;
                        }
                    })
                },
                |name, mut cursor| {
                    match name.name {
                        v if v == *crate::symbol::require => {
                            if on_call_requirement.is_some() {
                                self.error(item.args.span().unwrap(), |diag| {
                                    diag.help("property is specified more than once");
                                })?;
                            }

                            let range;
                            (range, cursor) = self.parse_irql_range(cursor)?;
                            on_call_requirement = Some(range);
                        }
                        v if v == *crate::symbol::always => {
                            if permanent_requirement.is_some() {
                                self.error(item.args.span().unwrap(), |diag| {
                                    diag.help("property is specified more than once");
                                })?;
                            }

                            let range;
                            (range, cursor) = self.parse_irql_range(cursor)?;
                            permanent_requirement = Some(range);
                        }
                        v if v == *crate::symbol::raise => {
                            if on_return_value.is_some() {
                                self.error(item.args.span().unwrap(), |diag| {
                                    diag.help("property is specified more than once");
                                })?;
                            }

                            let irql_value;
                            (irql_value, cursor) = self.parse_irql_value(cursor)?;
                            on_return_value = Some(irql_value);
                        }
                        _ => unreachable!(),
                    }

                    Ok(cursor)
                },
            )
        })?;

        if on_call_requirement.is_none()
            && permanent_requirement.is_none()
            && on_return_value.is_none()
        {
            self.error(item.args.span().unwrap(), |diag| {
                diag.help(
                    "at least one of `require`, `always`, or `raise` property must be specified",
                );
            })?;
        }

        if on_call_requirement.is_some() && permanent_requirement.is_some() {
            self.error(item.args.span().unwrap(), |diag| {
                diag.help("`require` and `always` can't be used together");
            })?;
        }

        Ok(Irql {
            on_call_requirement,
            permanent_requirement,
            on_return_value,
        })
    }

    fn parse(&self, attr: &ast::Attribute) -> Option<KlintAttribute> {
        let ast::AttrKind::Normal(normal_attr) = &attr.kind else {
            return None;
        };
        let item = &normal_attr.item;
        if item.path.segments[0].ident.name != *crate::symbol::klint {
            return None;
        };
        if item.path.segments.len() != 2 {
            self.tcx
                .node_span_lint(crate::INCORRECT_ATTRIBUTE, self.hir_id, attr.span, |lint| {
                    lint.primary_message("invalid klint attribute");
                });
            return None;
        }
        match item.path.segments[1].ident.name {
            v if v == *crate::symbol::preempt_count => Some(KlintAttribute::PreemptionCount(
                self.parse_preempt_count(attr, item).ok()?,
            )),
            v if v == *crate::symbol::drop_preempt_count => Some(
                KlintAttribute::DropPreemptionCount(self.parse_preempt_count(attr, item).ok()?),
            ),
            v if v == *crate::symbol::report_preempt_count => {
                Some(KlintAttribute::ReportPreeptionCount)
            }
            v if v == *crate::symbol::dump_mir => Some(KlintAttribute::DumpMir),
            v if v == *crate::symbol::irql => {
                Some(KlintAttribute::Irql(self.parse_irql(attr, item).ok()?))
            }
            _ => {
                self.tcx.node_span_lint(
                    crate::INCORRECT_ATTRIBUTE,
                    self.hir_id,
                    item.path.segments[1].span(),
                    |lint| {
                        lint.primary_message("unrecognized klint attribute");
                    },
                );
                None
            }
        }
    }
}

pub fn parse_klint_attribute(
    tcx: TyCtxt<'_>,
    hir_id: HirId,
    attr: &ast::Attribute,
) -> Option<KlintAttribute> {
    AttrParser { tcx, hir_id }.parse(attr)
}

memoize!(
    pub fn klint_attributes<'tcx>(
        cx: &AnalysisCtxt<'tcx>,
        hir_id: HirId,
    ) -> Lrc<Vec<KlintAttribute>> {
        let mut v = Vec::new();
        for attr in cx.hir().attrs(hir_id) {
            let Some(attr) = crate::attribute::parse_klint_attribute(cx.tcx, hir_id, attr) else {
                continue;
            };
            v.push(attr);
        }
        Lrc::new(v)
    }
);
