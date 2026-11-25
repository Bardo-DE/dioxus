//! Placement scans of fragment siblings during a fragment diff.
//!
//! Stable fragment children are diffed in place, in order. When a later child places a new
//! node and has no live edge of its own, placement walks up to the fragment and scans the
//! child's siblings. The siblings already diffed hold the new view in their mounts (and in
//! their nested fragment child mounts), so scanning them through the parent's old child
//! vnodes paired an old template with a new mount and indexed slots that do not exist.
use dioxus::prelude::*;
use dioxus_core::{generation, ScopeId};
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn earlier_sibling_nested_child_changes_template() {
    #[component]
    fn Comp() -> Element {
        rsx! { b {} }
    }

    // The tree is `div { [first, second] }`, a fragment of two children whose templates
    // never change, so both are diffed in place. Every render flips between:
    //
    //   even generations: first = [ p {} ],    second = (empty)
    //   odd generations:  first = [ Comp {} ], second = span {}
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        // A one-node fragment whose node changes template: diffing it gives the nested
        // node a new mount.
        let nested = if odd {
            rsx! { Comp {} }
        } else {
            rsx! { p {} }
        };
        let first = rsx! { {std::iter::once(nested)} };

        // No static element of its own, so inserting `span` scans back to `first`, which
        // has already been diffed.
        let second = rsx! {
            if odd {
                span {}
            }
        };

        rsx! {
            div { {[first, second].into_iter()} }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches_vdom(&dom);

    // Flip both ways twice; each render must match a fresh build of the same tree.
    for _ in 0..4 {
        dom.mark_dirty(ScopeId::APP);
        oracle.render(&mut dom);
        oracle.assert_matches_vdom(&dom);
    }
}

/// Placement walks up through a component root to a fragment child of a grandparent that is
/// itself mid-diff. That grandparent is three diff frames above the placing mount, so it must
/// still be found as an in-flight frame: reading it from the committed mount view pairs its old
/// vnode with child mounts an earlier sibling slot has already replaced.
#[test]
fn placement_scans_in_flight_grandparent_sibling_after_template_change() {
    #[component]
    fn Comp() -> Element {
        rsx! { b {} }
    }

    // No static element of its own: showing `span` has no local anchor, so placement lifts to
    // the fragment child that holds this component in `app`'s `div`.
    #[component]
    fn Late(show: bool) -> Element {
        rsx! {
            if show {
                span {}
            }
        }
    }

    // `div { {first} {second} }`, two adjacent fragment slots in one same-template vnode. Every
    // render flips between:
    //
    //   even generations: first = [ p {} ],    Late shows `span`
    //   odd generations:  first = [ Comp {} ], Late shows nothing
    //
    // Going odd -> even, `first` swaps its `Comp {}` vnode for a static `p {}` (a new mount, with
    // no dynamic slots) before `second` is diffed. `Late` then re-renders through its parent
    // (its prop changes), so its diff frame is nested under the fragment child's frame, which is
    // nested under `app`'s root frame: three deep, past what a two-frame context could hold.
    // Inserting `span` scans back over `first`, which must be read as the new `p {}`.
    fn app() -> Element {
        let odd = generation() % 2 == 1;

        let nested = if odd {
            rsx! { Comp {} }
        } else {
            rsx! { p {} }
        };
        let late = rsx! { Late { show: !odd } };

        rsx! {
            div {
                {std::iter::once(nested)}
                {std::iter::once(late)}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches_vdom(&dom);

    for _ in 0..4 {
        dom.mark_dirty(ScopeId::APP);
        oracle.render(&mut dom);
        oracle.assert_matches_vdom(&dom);
    }
}
