mod infrai_bridge;
mod shipment_guard;

use infrai_bridge::InfraiBridge;
use shipment_guard::{decide, HandlingDecision, ShipmentEvent};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = InfraiBridge::from_env()?;
    block_on(bridge.set_hard_cap("25.00", "monthly"))?;

    let event = ShipmentEvent {
        shipment_id: "SHP-204".into(),
        event: "delivery_exception".into(),
        proof_reference: Some("pod/SHP-204.jpg".into()),
        exception_note: Some("recipient unavailable".into()),
    };
    let decision = decide(&event, "25.00", true)?;
    match decision {
        HandlingDecision::AskModel { prompt } => {
            let answer = block_on(bridge.classify_exception(&prompt))?;
            println!("dispatch action: {answer}");
        }
        HandlingDecision::HoldForReview { reason } => println!("dispatch action: {reason}"),
    }
    let usage = block_on(bridge.usage_timeseries())?;
    println!("usage timeseries: {usage}");
    Ok(())
}

fn block_on<F: Future>(future: F) -> F::Output {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker { RawWaker::new(std::ptr::null(), &VTABLE) }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    let raw = RawWaker::new(std::ptr::null(), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw) };
    let mut context = Context::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(Box::new(future)) };
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
