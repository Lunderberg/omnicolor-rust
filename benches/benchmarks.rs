// -- Cargo.toml --
// [[bench]]
// name = "benchmarks"
// harness = false
//
// [dev-dependencies]
// criterion = {version = "0.3", features=['html_reports']}

use criterion::{criterion_group, criterion_main, Bencher, Criterion};

use omnicolor_rust::palettes::UniformPalette;
use omnicolor_rust::GrowthImageBuilder;

fn generate_flat_image(b: &mut Bencher) {
    let mut builder = GrowthImageBuilder::new();
    builder
        .add_layer(1920, 1080)
        .epsilon(5.0)
        .palette(UniformPalette);

    b.iter(|| {
        let mut image = builder.build().unwrap();
        image.fill_until_done();
    });
}

fn bench_flat_image(c: &mut Criterion) {
    let mut group = c.benchmark_group("Image-gen");
    group
        .noise_threshold(0.07)
        .sample_size(20)
        .sampling_mode(criterion::SamplingMode::Flat)
        .measurement_time(std::time::Duration::from_secs(120));

    group.bench_function("flat-image", generate_flat_image);

    group.finish();
}

criterion_group!(benches, bench_flat_image);
criterion_main!(benches);
