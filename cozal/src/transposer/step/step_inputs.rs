use std::any::TypeId;
use std::collections::BTreeMap;
use std::pin::Pin;

use archery::SharedPointerKind;
use futures_core::Future;
use type_erased_vec::TypeErasedVec;

use super::{sub_step_update_context::SubStepUpdateContext, InputState};
use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

pub struct StepInputs<T: Transposer, P: SharedPointerKind, Is> {
    pub time: T::Time,

    // these btreesets are all of different values. they are transmuted before use.
    inputs: BTreeMap<u64, StepInputsEntry<T, P, Is>>,
}

type HandlerFunction<T, P, Is> = for<'a> fn(
    time: <T as Transposer>::Time,
    &'a mut T,
    &'a mut SubStepUpdateContext<'_, T, P, Is>,
    &'a TypeErasedVec,
) -> Pin<Box<dyn 'a + Future<Output = ()>>>;

struct StepInputsEntry<T: Transposer, P: SharedPointerKind, Is> {
    // keep this sorted
    values: TypeErasedVec,
    input_type_id: TypeId,
    handler: HandlerFunction<T, P, Is>,
}

impl<T: Transposer, P: SharedPointerKind, Is: InputState<T>> StepInputsEntry<T, P, Is> {
    fn new<I: TransposerInput<Base = T>>() -> Self
    where
        T: TransposerInputEventHandler<I>,
    {
        // Self {
        //     values: TypeErasedVec::new::<I::InputEvent>(),
        //     input_type_id: TypeId::of::<I>(),
        //     handler: |time, t, cx, set| {
        //         // SAFETY: this came from the assignment to values, which erased the I::InputEvent type
        //         let set = unsafe { set.get::<I::InputEvent>() };
        //         Box::pin(async move {
        //             for i in set.iter() {
        //                 T::handle_input_event(t, i, cx).await;
        //                 cx.metadata.last_updated.time = time;
        //                 cx.metadata.last_updated.index += 1;
        //             }
        //         })
        //     },
        // }

        todo!()
    }

    fn add_input<I: TransposerInput<Base = T>>(&mut self, time: T::Time, input: I::InputEvent)
    where
        T: TransposerInputEventHandler<I>,
    {
        if TypeId::of::<I>() != self.input_type_id {
            panic!()
        }

        // SAFETY: this matches the type because I has a TypeId that matches the one that created it.
        let set = unsafe { self.values.get_mut() };

        // let i =
        //     set.partition_point(|existing| T::sort_input_events(time, &input, existing).is_lt());
        let i = todo!();

        set.insert(i, input);
    }
}

impl<T: Transposer, P: SharedPointerKind, Is: InputState<T>> StepInputs<T, P, Is> {
    pub async fn handle(&self, transposer: &mut T, cx: &mut SubStepUpdateContext<'_, T, P, Is>) {
        for (_, i) in self.inputs.iter() {
            (i.handler)(self.time, transposer, cx, &i.values).await;
        }
    }

    pub fn add_event<I: TransposerInput<Base = T>>(&mut self, event: I::InputEvent)
    where
        T: TransposerInputEventHandler<I>,
    {
        let step_inputs_entry = match self.inputs.entry(I::SORT) {
            std::collections::btree_map::Entry::Vacant(v) => v.insert(StepInputsEntry::new()),
            std::collections::btree_map::Entry::Occupied(o) => o.into_mut(),
        };

        step_inputs_entry.add_input(self.time, event);
    }

    pub fn time(&self) -> T::Time {
        self.time
    }
}
