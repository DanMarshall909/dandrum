use super::TimerRegistration;
use clack_extensions::timer::TimerId;
use clack_plugin::prelude::PluginError;
use std::cell::RefCell;

#[test]
fn registration_owns_disabled_tokens_across_failed_rollback_and_retries_without_duplicate_registration()
 {
    let timer = TimerRegistration::default();
    let registrations = RefCell::new(Vec::new());
    let removals = RefCell::new(Vec::new());
    let refused = || Err(PluginError::Message("host refuses timer"));
    assert!(
        timer
            .prepare(refused, |_| panic!("no prior token"))
            .is_err()
    );
    let first = TimerId(7);
    let second = TimerId(8);
    assert!(
        timer
            .prepare(
                || {
                    assert!(!timer.is_live(first));
                    assert!(
                        timer
                            .prepare(
                                || panic!("registering prevents recursion"),
                                |_| panic!("no cleanup during registering")
                            )
                            .is_err()
                    );
                    registrations.borrow_mut().push(first);
                    Ok(first)
                },
                |_| panic!("initial registration has no cleanup")
            )
            .unwrap()
    );
    assert!(
        !timer.is_live(first),
        "token is disabled until configuration commit"
    );
    assert!(
        timer
            .remove(|id| {
                removals.borrow_mut().push((id, false));
                Err(PluginError::Message("rollback refused"))
            })
            .is_err()
    );
    assert!(!timer.is_live(first));
    assert!(
        timer
            .prepare(
                || panic!("failed cleanup must prevent a second registration"),
                |id| {
                    assert_eq!(id, first);
                    removals.borrow_mut().push((id, false));
                    Err(PluginError::Message("retry refused"))
                }
            )
            .is_err()
    );
    assert!(
        timer
            .prepare(
                || {
                    registrations.borrow_mut().push(second);
                    Ok(second)
                },
                |id| {
                    assert_eq!(id, first);
                    removals.borrow_mut().push((id, true));
                    Ok(())
                }
            )
            .unwrap()
    );
    timer.enable().unwrap();
    assert!(timer.is_live(second));
    assert!(!timer.is_live(first));
    assert!(!timer.is_live(TimerId(99)));
    assert!(
        !timer
            .prepare(
                || panic!("live token is reused"),
                |_| panic!("live token is retained")
            )
            .unwrap()
    );
    timer.disable();
    assert!(!timer.is_live(second));
    timer
        .remove(|id| {
            assert_eq!(id, second);
            assert!(!timer.is_live(id));
            assert!(
                timer
                    .remove(|_| panic!("removing prevents recursive cancellation"))
                    .is_err()
            );
            removals.borrow_mut().push((id, true));
            Ok(())
        })
        .unwrap();
    timer
        .remove(|_| panic!("successful removal occurs only once"))
        .unwrap();
    assert!(
        timer.enable().is_err(),
        "empty registration cannot become live"
    );
    assert_eq!(*registrations.borrow(), [first, second]);
    assert_eq!(
        *removals.borrow(),
        [
            (first, false),
            (first, false),
            (first, true),
            (second, true)
        ]
    );
}
