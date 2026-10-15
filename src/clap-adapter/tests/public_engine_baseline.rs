mod common;

#[test]
fn signed_gain_root_control_baseline() {
    let output = common::direct_engine(common::GAIN, 16, 48000, &[], &[(5, 0, 0.25), (9, 0, 0.75)]);
    for (channel, expected) in output
        .iter()
        .zip([[-0.25, -0.125, -0.375], [0.125, 0.0625, 0.1875]])
    {
        assert_eq!(&channel[..5], &[expected[0]; 5]);
        assert_eq!(&channel[5..9], &[expected[1]; 4]);
        assert_eq!(&channel[9..], &[expected[2]; 7]);
    }
}

#[test]
fn tb303_timed_notes_and_root_control_baseline() {
    let notes = [
        (5, [0x90, 60, 110]),
        (24, [0x90, 64, 90]),
        (30, [0x80, 60, 0]),
        (48, [0x80, 64, 0]),
    ];
    let output = common::direct_engine(common::TB303, 64, 48000, &notes, &[(16, 0, 0.2)]);
    for channel in &output {
        assert_eq!(&channel[..5], &[0.0; 5]);
        for (frame, expected) in [
            (6, -0.0000019055),
            (12, -0.0002156532),
            (20, -0.0014245948),
            (31, -0.0035819469),
        ] {
            assert!(
                (channel[frame] - expected).abs() <= 1.0e-8,
                "literal TB303 sample at{frame}: {expected} vs {}",
                channel[frame]
            );
        }
        assert_eq!(channel[50], 0.0);
        assert_eq!(channel[63], 0.0);
    }
}
