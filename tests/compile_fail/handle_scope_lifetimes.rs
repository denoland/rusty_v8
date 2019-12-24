extern crate rusty_v8 as v8;

pub fn main() {
  let mut isolate: v8::scope::Entered<'_, v8::HandleScope> = mock();

  {
    let mut hs = v8::EscapableHandleScope::new(&mut isolate);
    let hs = hs.enter();
    let _fail = v8::EscapableHandleScope::new(&mut isolate);
    let _local = v8::Integer::new(hs, 123);
  }

  {
    let mut hs1 = v8::EscapableHandleScope::new(&mut isolate);
    let hs1 = hs1.enter();
    let _local1 = v8::Integer::new(hs1, 123);

    let mut hs2 = v8::EscapableHandleScope::new(hs1);
    let hs2 = hs2.enter();
    let _fail = v8::Integer::new(hs1, 123);
    let _local2 = v8::Integer::new(hs2, 123);
    let _local3 = v8::Integer::new(hs2, 123);
  }

  let _leak1 = {
    let mut hs = v8::EscapableHandleScope::new(&mut isolate);
    let hs = hs.enter();
    v8::Integer::new(hs, 456)
  };

  let _leak = {
    let mut hs = v8::EscapableHandleScope::new(&mut isolate);
    hs.enter()
  };
}

fn mock<T>() -> T {
  unimplemented!()
}

fn access<T>(_value: T) {}
