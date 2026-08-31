//! Field-exact conversion between the lifecycle and native epoch-1 roots.

use vibe_wire::generated::lifecycle::e1::context::Context;
use vibe_wire::generated::lifecycle::e1::reply::Reply;

pub(super) fn native_context(
    context: &Context,
) -> vibe_wire::generated::native::e1::context::Context {
    vibe_wire::generated::native::e1::context::Context {
        artifacts: context.artifacts.clone(),
        envelope: context.envelope,
        execution: context.execution.clone(),
        io: context.io.clone(),
        point: context.point.clone(),
        project: context.project.clone(),
        run: context.run.clone(),
        world: context.world.clone(),
        slot_target: context.slot_target.clone(),
    }
}

pub(super) fn lifecycle_reply(reply: vibe_wire::generated::native::e1::reply::Reply) -> Reply {
    Reply {
        artifacts: reply.artifacts,
        envelope: reply.envelope,
        status: reply.status,
        tasks: Vec::new(),
        message: reply.message,
    }
}
