use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_in_test_function;

use rustc_hir as hir;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, GenericParam, Generics, HirId, ImplItem, ImplItemKind, TraitItem, TraitItemKind};
use rustc_lint::LateContext;
use rustc_span::symbol::Ident;
use rustc_span::{BytePos, Span};

use super::IMPL_TRAIT_IN_PARAMS;

fn report(
    cx: &LateContext<'_>,
    param: &GenericParam<'_>,
    ident: &Ident,
    generics: &Generics<'_>,
    first_param_span: Span,
) {
    // No generics with nested generics, and no generics like FnMut(x)
    span_lint_and_then(
        cx,
        IMPL_TRAIT_IN_PARAMS,
        param.span,
        "`impl Trait` used as a function parameter",
        |diag| {
            if let Some(gen_span) = generics.span_for_param_suggestion() {
                // If there's already a generic param with the same bound, do not lint **this** suggestion.
                diag.span_suggestion_with_style(
                    gen_span,
                    "add a type parameter",
                    format!(", {{ /* Generic name */ }}: {}", &param.name.ident().as_str()[5..]),
                    rustc_errors::Applicability::HasPlaceholders,
                    rustc_errors::SuggestionStyle::ShowAlways,
                );
            } else {
                diag.span_suggestion_with_style(
                    Span::new(
                        first_param_span.lo() - rustc_span::BytePos(1),
                        ident.span.hi(),
                        ident.span.ctxt(),
                        ident.span.parent(),
                    ),
                    "add a type parameter",
                    format!("<{{ /* Generic name */ }}: {}>", &param.name.ident().as_str()[5..]),
                    rustc_errors::Applicability::HasPlaceholders,
                    rustc_errors::SuggestionStyle::ShowAlways,
                );
            }
        },
    );
}

pub(super) fn check_fn<'tcx>(cx: &LateContext<'_>, kind: &'tcx FnKind<'_>, body: &'tcx Body<'_>, hir_id: HirId) {
    if let FnKind::ItemFn(ident, generics, _) = kind
        && cx.tcx.visibility(cx.tcx.hir().body_owner_def_id(body.id())).is_public()
        && !is_in_test_function(cx.tcx, hir_id)
    {
        for param in generics.params {
            if param.is_impl_trait() {
                report(cx, param, ident, generics, body.params[0].span);
            };
        }
    }
}

pub(super) fn check_impl_item(cx: &LateContext<'_>, impl_item: &ImplItem<'_>) {
    if let ImplItemKind::Fn(_, body_id) = impl_item.kind
        && let hir::Node::Item(item) = cx.tcx.hir().get_parent(impl_item.hir_id())
        && let hir::ItemKind::Impl(impl_) = item.kind
        && let hir::Impl { of_trait, .. } = *impl_
        && of_trait.is_none()
        && let body = cx.tcx.hir().body(body_id)
        && cx.tcx.visibility(cx.tcx.hir().body_owner_def_id(body.id())).is_public()
        && !is_in_test_function(cx.tcx, impl_item.hir_id())
    {
        for param in impl_item.generics.params {
            if param.is_impl_trait() {
                report(cx, param, &impl_item.ident, impl_item.generics, body.params[0].span);
            }
        }
    }
}

pub(super) fn check_trait_item(cx: &LateContext<'_>, trait_item: &TraitItem<'_>, avoid_breaking_exported_api: bool) {
    if !avoid_breaking_exported_api
        && let TraitItemKind::Fn(_, _) = trait_item.kind
        && let hir::Node::Item(item) = cx.tcx.hir().get_parent(trait_item.hir_id())
        // ^^ (Will always be a trait)
        && !item.vis_span.is_empty() // Is public
        && !is_in_test_function(cx.tcx, trait_item.hir_id())
    {
        for param in trait_item.generics.params {
            if param.is_impl_trait() {
                let sp = trait_item.ident.span.with_hi(trait_item.ident.span.hi() + BytePos(1));
                report(cx, param, &trait_item.ident, trait_item.generics, sp.shrink_to_hi());
            }
        }
    }
}
