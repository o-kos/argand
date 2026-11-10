use super::*;

#[test]
fn progress_counts_iq_pairs_and_never_waits_for_a_full_reply_queue() {
    use argand_core::{Domain, SampleFormat, SampleType};
    use argand_io::testutil::{TempDir, iq_tone, write_wav};
    let dir = TempDir::new("navigation-progress");
    let path = write_wav(
        &dir.join("iq.wav"),
        SampleType::new(Domain::Iq, SampleFormat::F32),
        48000,
        &iq_tone(4096, 48000.0, 6000.0, 0.5),
        1.0,
    );
    let mut input = argand_io::open(&path, &OpenHints::default()).unwrap();
    let (analyst, updates, _start) = prepare(
        path,
        OpenHints::default(),
        Default::default(),
        crate::release::lease().0,
    );
    let analysis = crate::settings::Settings::from_config(&crate::config::Config::default())
        .analysis_request(input.meta(), 10, 10);
    analyst.request(analysis);
    let mut request = analyst.mailbox.latest().unwrap();
    request.replacement = true;
    let (_sender, requests) = async_channel::bounded(1);
    let (outgoing, receiver) = async_channel::bounded(1);
    let replies = Replies {
        requests: &requests,
        updates: &outgoing,
        mailbox: &analyst.mailbox,
    };
    let mut source = Source::new(input.as_mut(), request, &replies);
    let mut buffer = [0.0; 200];
    for _ in 0..2 {
        source.last = Instant::now() - Duration::from_millis(60);
        assert_eq!(source.read(&mut buffer).unwrap(), 200);
    }
    let delivery = receiver.try_recv().unwrap();
    assert!(analyst.accepts(&delivery));
    assert!(matches!(
        delivery.update,
        Update::Progress {
            done: 100,
            total: 4096
        }
    ));
    assert!(receiver.is_empty());
    drop(updates);
}
