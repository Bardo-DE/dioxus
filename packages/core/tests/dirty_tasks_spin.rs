//! Regression test: cancelling a queued task must not leave an empty dirty-task entry that makes
//! `poll_tasks` spin forever (upstream fix: DioxusLabs/dioxus#5554).

use std::{cell::Cell, rc::Rc, sync::mpsc, thread, time::Duration};

use dioxus::prelude::*;
use dioxus_core::Task;

fn app() -> Element {
    use_hook(|| {
        let victim: Rc<Cell<Option<Task>>> = Rc::default();

        // Polled first: cancels the victim while the victim is still queued but not yet polled.
        let slot = victim.clone();
        spawn(async move {
            if let Some(task) = slot.take() {
                task.cancel();
            }
        });

        victim.set(Some(spawn(std::future::pending::<()>())));
    });

    rsx! {}
}

#[test]
fn cancelling_a_queued_task_does_not_spin() {
    let (done, finished) = mpsc::channel();

    // Run on a separate thread so a regression fails the test instead of hanging it.
    thread::spawn(move || {
        let mut dom = VirtualDom::new(app);
        dom.rebuild(&mut dioxus_core::NoOpMutations);
        dom.process_events();
        let _ = done.send(());
    });

    assert!(
        finished.recv_timeout(Duration::from_secs(5)).is_ok(),
        "poll_tasks spun forever on an empty dirty-task entry"
    );
}
