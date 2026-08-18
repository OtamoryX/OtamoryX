use super::*;

#[test]
fn topic_coverage_uses_candidate_topics_as_the_denominator() {
    let mut sets = TopicSets::default();
    sets.candidate
        .extend(["a".to_string(), "b".to_string(), "c".to_string()]);
    sets.exposed.extend(["a".to_string(), "c".to_string()]);
    sets.exploration.insert("b".to_string());
    let metric = coverage_metric(&sets);
    assert_eq!(metric.candidate_topic_count, 3);
    assert_eq!(metric.exposed_topic_count, 2);
    assert_eq!(metric.exploration_topic_count, 1);
    assert!((metric.exposure_coverage - 2.0 / 3.0).abs() < f64::EPSILON);
    assert!((metric.exploration_coverage - 1.0 / 3.0).abs() < f64::EPSILON);
}

#[test]
fn manual_deletes_are_normalized_by_opens() {
    let mut metric = RandomRecommendationMetric {
        opened: 4,
        manual_deletes: 1,
        effective_reads: 2,
        ..Default::default()
    };
    finalize_metric(&mut metric);
    assert_eq!(metric.effective_read_rate, 0.5);
    assert_eq!(metric.manual_deletes_per_100_opens, 25.0);
}
