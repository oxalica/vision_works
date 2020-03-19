use ocl::{ProQue, Result};

const DIM: usize = 8;
const KERNEL_SRC: &str = r#"
    kernel void square (int x, global int *out) {
        out[get_global_id(0)] += x * x;
    }
"#;

fn main() -> Result<()> {
    let pro_que = ProQue::builder().src(KERNEL_SRC).dims(DIM).build()?;
    let buf = pro_que.create_buffer::<i32>()?;
    buf.write(&[0i32; DIM][..]).enq()?;
    let kernel = pro_que
        .kernel_builder("square")
        .arg(5i32)
        .arg(&buf)
        .build()?;
    unsafe { kernel.enq()? };

    let mut out = vec![0i32; DIM];
    buf.read(&mut out).enq()?;
    println!("{:?}", out);
    assert_eq!(out, [5 * 5; 8]);

    Ok(())
}
