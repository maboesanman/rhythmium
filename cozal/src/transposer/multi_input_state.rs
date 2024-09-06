use std::{collections::{HashMap, VecDeque}, future::Future, ptr::NonNull};

use futures_channel::{oneshot::{channel, Sender, Receiver}};
use parking_lot::{Mutex, RwLock};

use super::TransposerInput;

/// This doesn't work if any of the items in the tuple are the same type.
/// 
/// This is based off the principle demonstrated in this playground:
/// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=87aacb9d6a9a0558e9d376e4cef624bd
macro_rules! multi_input_state {
    ($name:ident($transposer:ty, $($t:ty),+)) => {
        pub struct $name {
            requested_types: VecDeque<TypeId>,
            inner: ($(parking_lot::RwLock<SingleInputStateInner<<$t>>),*)
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    inner: RwLock::new(($(multi_input_state!{@empty $t},)*))
                }
            }

            pub fn get_next_requested_input(&mut self) -> Option<TypeId> {
                self.requested_types.pop_front()
            }
        }

        multi_input_state!{@select $name ; $($t),+}
    };
    (@select $name:ident $($prev:ty),* ; $curr:ty $(, $next:ty)*) => {
        unsafe impl StateRetriever<$curr> for $name
        where $curr: TransposerInput
        {
            fn get_input_state(&self, _: $curr) -> Receiver<NonNull<$curr::InputState>> {
                let (send, recv) = channel();
                let read = self.inner.read();
                let ($(multi_input_state!{@_ $prev},)* single, ..) = &(*read);
                match single {
                    SingleInputStateInner::Empty => {
                        drop(read);
                        let mut write = self.inner.write();
                        let ($(multi_input_state!{@_ $prev},)* single, ..) = &mut (*write);
                        single = SingleInputStateInner::Requested(vec![send]);
                    }
                    SingleInputStateInner::Requested(_) => {
                        drop(read);
                        let mut write = self.inner.write();
                        let ($(multi_input_state!{@_ $prev},)* single, ..) = &mut (*write);
                        if let SingleInputStateInner::Requested(vec) = single {
                            vec.push(send)
                        } else {
                            unreachable!()
                        }
                    }
                    SingleInputStateInner::Full(input_state) => {
                        let _ = send.send((&**input_state).into());
                    }
                }
        
                recv
            }
        }
        multi_input_state!{@select $name $($prev,)* $curr ; $($next),*}
    };
    (@select $name:ident $($done:ty),* ; ) => {};
    (@_ $dummy:ty) => {_};
    (@empty $dummy:ty) => { SingleInputStateInner::Empty };
}

// enum SingleInputStateInner<I: TransposerInput> {
//     Empty,
//     Requested(Vec<Sender<NonNull<I::InputState>>>),
//     Full(Box<I::InputState>),
// }

struct SingleInputStateInner<I: TransposerInput> {
    requested_inputs: VecDeque<I>,
    input_states: HashMap<I, InputStateStatus<I::InputState>>,
}

enum InputStateStatus<S> {
    Requested(Vec<Sender<NonNull<S>>>),
    Resolved(NonNull<S>),
}

impl<I: TransposerInput> SingleInputStateInner<I> {
    fn get_state(this: RwLock<Self>, input: I) -> (bool, impl Future<Output = NonNull<I::InputState>>) {
        let read = this.read();
        let input_state_status = read.input_states.get(&input);
        let resolved_state = match input_state_status {
            Some(InputStateStatus::Resolved(input_state)) => Some(*input_state),
            _ => None,
        };
        drop(read);

        enum FutureBehavior<S> {
            Ready(NonNull<S>),
            Pending(Receiver<NonNull<S>>),
        }

        let (first_request, future_behavior) = match resolved_state {
            Some(_) => (false, FutureBehavior::Ready(resolved_state.unwrap())),
            None => {
                let mut write = this.write();
                match write.input_states.get_mut(&input) {
                    Some(InputStateStatus::Resolved(input_state)) => {
                        (false, FutureBehavior::Ready(*input_state))
                    },
                    Some(InputStateStatus::Requested(vec)) => {
                        let (send, recv) = channel();
                        vec.push(send);
                        (false, FutureBehavior::Pending(recv))
                    },
                    None => {
                        let (send, recv) = channel();
                        write.input_states.insert(input, InputStateStatus::Requested(vec![send]));
                        (true, FutureBehavior::Pending(recv))
                    },
                }
            },
        };

        let fut = async move {
            match future_behavior {
                FutureBehavior::Ready(state) => state,
                FutureBehavior::Pending(recv) => recv.await.unwrap(),
            }
        };

        (first_request, fut)
    }

    fn set_state(this: RwLock<Self>, input: I, state: I::InputState) -> Result<(), I::InputState> {
        let mut write = this.write();
        let input_state_status = write.input_states.remove(&input);
        let (result, ptr) = match input_state_status {
            Some(InputStateStatus::Resolved(ptr)) => (Err(state), ptr),
            Some(InputStateStatus::Requested(vec)) => {
                let state = Box::new(state);
                let ptr: NonNull<_> = (&*state).into();
                for send in vec {
                    let _ = send.send(ptr);
                }
                (Ok(()), ptr)
            },
            None => {
                let state = Box::new(state);
                let ptr: NonNull<_> = (&*state).into();
                (Ok(()), ptr)
            },
        };

        write.input_states.insert(input, InputStateStatus::Resolved(ptr));

        result
    }
}

// trait StateSenderRetriever<I: TransposerInput> {
//     pub fn set_state(&self, _: $curr, state: $curr::InputState) -> Result<(), $curr::InputState> {
//         let mut inner = self.inner.write();
//         let ($(tuple_input_state_manager!{@_ $prev},)* single, ..) = &mut (*inner);
//         let senders = match core::mem::replace(single, SingleInputStateInner::Empty) {
//             SingleInputStateInner::Empty => Vec::new(),
//             SingleInputStateInner::Requested(vec) => vec,
//             SingleInputStateInner::Full(s) => {
//                 *single = SingleInputStateInner::Full(s);
//                 return Err(state);
//             }
//         };
//         let state = Box::new(state);
    
//         for send in senders.into_iter() {
//             let ptr: NonNull<_> = (&*state).into();
//             let _ = send.send(ptr);
//         }
    
//         *single = SingleInputStateInner::Full(state);
    
//         Ok(())
//     }
// }

