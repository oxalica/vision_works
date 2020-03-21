use ocl::ProQue;
use std::ops::Range;

const KERNEL_SRC: &str = include_str!("./add_mul.cl");

fn arr_add(a: &[i32], b: &[i32], c: &mut [i32]) {
    debug_assert_eq!(a.len(), b.len());
    for ((x, y), z) in a.iter().zip(b).zip(c) {
        *z = *x + *y;
    }
}

fn mat_mul(n: usize, a: &[i32], b: &[i32], out: &mut [i32]) {
    debug_assert_eq!(a.len(), n * n);
    debug_assert_eq!(b.len(), n * n);
    debug_assert_eq!(out.len(), n * n);
    debug_assert!(out.iter().all(|&x| x == 0));
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                out[i * n + k] += a[i * n + j] * b[j * n + k];
            }
        }
    }
}

fn arr_add_cl(n: usize) -> impl FnMut(&[i32], &[i32], &mut [i32]) {
    let pro_que = ProQue::builder().src(KERNEL_SRC).dims(n).build().unwrap();
    let buf1 = pro_que.create_buffer::<i32>().unwrap();
    let buf2 = pro_que.create_buffer::<i32>().unwrap();
    let kernel = pro_que
        .kernel_builder("arr_add")
        .arg(&buf1)
        .arg(&buf2)
        .build()
        .unwrap();

    move |a, b, c| {
        buf1.write(a).enq().unwrap();
        buf2.write(b).enq().unwrap();
        unsafe { kernel.enq().unwrap() };
        buf1.read(c).enq().unwrap();
    }
}

fn mat_mul_cl(n: usize) -> impl FnMut(&[i32], &[i32], &mut [i32]) {
    let pro_que = ProQue::builder()
        .src(KERNEL_SRC)
        .dims((n, n))
        .build()
        .unwrap();
    let buf1 = pro_que.create_buffer::<i32>().unwrap();
    let buf2 = pro_que.create_buffer::<i32>().unwrap();
    let buf3 = pro_que.create_buffer::<i32>().unwrap();
    let kernel = pro_que
        .kernel_builder("mat_mul")
        .arg(n as i32)
        .arg(&buf1)
        .arg(&buf2)
        .arg(&buf3)
        .build()
        .unwrap();

    move |a, b, c| {
        buf1.write(a).enq().unwrap();
        buf2.write(b).enq().unwrap();
        unsafe { kernel.enq().unwrap() };
        buf3.read(c).enq().unwrap();
    }
}

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn rand_vec(n: usize, range: std::ops::Range<i32>) -> Vec<i32> {
    use rand::prelude::*;
    let mut rng = thread_rng();
    (0..n)
        .map(|_| rng.gen_range(range.start, range.end))
        .collect()
}

const SMALL_ADD_SIZE: usize = 1000;
const LARGE_ADD_SIZE: usize = 10_000_000;
const SMALL_MAT_MUL_SIZE: usize = 10;
const LARGE_MAT_MUL_SIZE: usize = 300;
const ADD_RANGE: Range<i32> = 0..(1 << 30);
const MAT_MUL_RANGE: Range<i32> = 0..(1 << 10);

fn bench_cpu(c: &mut Criterion) {
    c.bench_function("cpu_array_add_small", |ben| {
        let a = rand_vec(SMALL_ADD_SIZE, ADD_RANGE);
        let b = rand_vec(SMALL_ADD_SIZE, ADD_RANGE);
        let mut c = vec![0; SMALL_ADD_SIZE];
        ben.iter(|| arr_add(black_box(&a), black_box(&b), &mut c));
    });
    c.bench_function("cpu_array_add_large", |ben| {
        let a = rand_vec(LARGE_ADD_SIZE, ADD_RANGE);
        let b = rand_vec(LARGE_ADD_SIZE, ADD_RANGE);
        let mut c = vec![0; LARGE_ADD_SIZE];
        ben.iter(|| arr_add(black_box(&a), black_box(&b), &mut c));
    });

    c.bench_function("cpu_mat_mul_small", |ben| {
        let a = rand_vec(SMALL_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let b = rand_vec(SMALL_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let mut c = vec![0; SMALL_MAT_MUL_SIZE.pow(2)];
        ben.iter(|| mat_mul(SMALL_MAT_MUL_SIZE, black_box(&a), black_box(&b), &mut c));
    });
    c.bench_function("cpu_mat_mul_large", |ben| {
        let a = rand_vec(LARGE_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let b = rand_vec(LARGE_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let mut c = vec![0; LARGE_MAT_MUL_SIZE.pow(2)];
        ben.iter(|| mat_mul(LARGE_MAT_MUL_SIZE, black_box(&a), black_box(&b), &mut c));
    });
}

fn bench_opencl(c: &mut Criterion) {
    c.bench_function("opencl_array_add_small", |ben| {
        let a = rand_vec(SMALL_ADD_SIZE, ADD_RANGE);
        let b = rand_vec(SMALL_ADD_SIZE, ADD_RANGE);
        let mut c = vec![0; SMALL_ADD_SIZE];
        let mut f = arr_add_cl(SMALL_ADD_SIZE);
        ben.iter(|| f(black_box(&a), black_box(&b), &mut c));
    });
    c.bench_function("opencl_array_add_large", |ben| {
        let a = rand_vec(LARGE_ADD_SIZE, ADD_RANGE);
        let b = rand_vec(LARGE_ADD_SIZE, ADD_RANGE);
        let mut c = vec![0; LARGE_ADD_SIZE];
        let mut f = arr_add_cl(LARGE_ADD_SIZE);
        ben.iter(|| f(black_box(&a), black_box(&b), &mut c));
    });

    c.bench_function("opencl_mat_mul_small", |ben| {
        let a = rand_vec(SMALL_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let b = rand_vec(SMALL_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let mut c = vec![0; SMALL_MAT_MUL_SIZE.pow(2)];
        let mut f = mat_mul_cl(SMALL_MAT_MUL_SIZE);
        ben.iter(|| f(black_box(&a), black_box(&b), &mut c));
    });
    c.bench_function("opencl_mat_mul_large", |ben| {
        let a = rand_vec(LARGE_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let b = rand_vec(LARGE_MAT_MUL_SIZE.pow(2), MAT_MUL_RANGE);
        let mut c = vec![0; LARGE_MAT_MUL_SIZE.pow(2)];
        let mut f = mat_mul_cl(LARGE_MAT_MUL_SIZE);
        ben.iter(|| f(black_box(&a), black_box(&b), &mut c));
    });
}

criterion_group!(benches, bench_cpu, bench_opencl);
criterion_main!(benches);
