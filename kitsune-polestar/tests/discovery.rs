use std::sync::{atomic::AtomicBool, Arc};

use kitsune_p2p::{
    agent_store::{AgentInfoInner, AgentInfoSigned},
    dependencies::{
        kitsune_p2p_fetch::FetchPool,
        kitsune_p2p_types::{
            box_fut,
            config::KitsuneP2pTuningParams,
            dependencies::futures::{self},
            tx2::tx2_utils::Share,
            GossipType,
        },
    },
    event::KitsuneP2pEvent,
    gossip::sharded_gossip::{
        store::AgentInfoSession, Initiate, ShardedGossipLocal, ShardedGossipLocalState,
        ShardedGossipWire,
    },
    metrics::MetricsSync,
    KitsuneSpace, MockKitsuneP2pEventHandler, NodeCert,
};
use polestar::prelude::*;
use proptest::test_runner::TestRunner;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "too much work to make kitsune fuzzable"]
async fn discovery() {
    let mut gen = TestRunner::default();
    let n1: NodeCert = gen.generate().unwrap();
    // let n2: NodeCert = gen.generate().unwrap();

    let gossip = make_gossip(&mut gen, GossipType::Recent).await;
    let mut agent_info_session = AgentInfoSession::default();

    let r1 = {
        let msg = ShardedGossipWire::Initiate(Initiate {
            intervals: vec![gen.generate().unwrap()],
            id: gen.generate().unwrap(),
            agent_list: vec![simple_agent(&mut gen)],
        });
        let _out = gossip
            .process_incoming(n1.clone(), msg, &mut agent_info_session)
            .await
            .unwrap();
        gossip
            .inner
            .share_mut(|s, _| Ok(s.round_map.get(&n1).cloned().unwrap()))
    };

    let r2 = {
        let msg = ShardedGossipWire::Agents(gen.generate().unwrap());
        gossip
            .process_incoming(n1.clone(), msg, &mut agent_info_session)
            .await
            .unwrap();

        gossip
            .inner
            .share_mut(|s, _| Ok(s.round_map.get(&n1).cloned().unwrap()))
    };

    println!(
        "{}",
        prettydiff::text::diff_lines(&format!("{:#?}", r1), &format!("{:#?}", r2))
    );

    let r3 = {
        let msg = ShardedGossipWire::MissingAgents(gen.generate().unwrap());
        gossip
            .process_incoming(n1.clone(), msg, &mut agent_info_session)
            .await
            .unwrap();

        gossip
            .inner
            .share_mut(|s, _| Ok(s.round_map.get(&n1).cloned().unwrap()))
    };

    println!(
        "{}",
        prettydiff::text::diff_lines(&format!("{:#?}", r2), &format!("{:#?}", r3))
    );
}

fn simple_agent(gen: &mut impl Generator) -> AgentInfoSigned {
    let mut a: AgentInfoInner = gen.generate().unwrap();
    a.url_list = vec![gen.generate().unwrap()];
    AgentInfoSigned(Arc::new(a))
}

async fn make_gossip(gen: &mut impl Generator, gossip_type: GossipType) -> ShardedGossipLocal {
    // let (sender, recv) = futures::channel::mpsc::channel(1000000);
    let tuning_params = KitsuneP2pTuningParams::default();
    let space: KitsuneSpace = gen.generate().unwrap();
    let metrics = MetricsSync::default();
    let fetch_pool = FetchPool::new_bitwise_or();
    let mut state = ShardedGossipLocalState::new(metrics);
    state.local_agents.insert(gen.generate().unwrap());

    let mut mock = MockKitsuneP2pEventHandler::new();
    mock.expect_handle_query_agents()
        .returning(|_| Ok(box_fut(Ok(vec![]))));
    let sender = build_event_handler(mock).await;
    let host_api = kitsune_p2p::HostStub::new().legacy(sender);

    ShardedGossipLocal {
        tuning_params,
        space: Arc::new(space),
        host_api,
        inner: Share::new(state),
        gossip_type,
        closing: AtomicBool::new(false),
        fetch_pool,
    }
}

async fn build_event_handler(
    m: MockKitsuneP2pEventHandler,
) -> futures::channel::mpsc::Sender<KitsuneP2pEvent> {
    let b = kitsune_p2p::dependencies::kitsune_p2p_types::dependencies::ghost_actor::actor_builder::GhostActorBuilder::new();
    let (evt_sender, r) = futures::channel::mpsc::channel::<KitsuneP2pEvent>(4096);
    b.channel_factory().attach_receiver(r).await.unwrap();
    tokio::task::spawn(b.spawn(m));
    evt_sender
}
