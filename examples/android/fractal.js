// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
function DrawFrame(frameLen) {
  const u8 = new Uint8Array(frameLen);
  const width = 800;
  const height = 800;

  let x = y = 0;

  for (let i = 0; i < u8.byteLength; i += 4) {
    if (x == width) {
      y++;
      x = 0;
    }

    x += 1;
    let r = Math.floor(0.3 * x);
    let b = Math.floor(0.3 * y);

    u8.set([r, 0x00, b, 0xff], i);
  }

  let scale_x = 3.0 / width;
  let scale_y = 3.0 / height;

  for (let x = 0; x < width; x++) {
    for (let y = 0; y < height; y++) {
      let cx = y * scale_x - 1.5;
      let cy = x * scale_y - 1.5;

      let c = new Complex(-0.4, 0.6);
      let z = new Complex(cx, cy);

      let i = 0;
      while (i < 100 && z.abs() < 2) {
        z = z.mul(z).add(c);
        i++;
      }

      u8.set([0x00, i, 0x00, 0xff], (y * width + x) * 4);
    }
  }

  return u8.buffer;
}

class Complex {
  constructor(real, imag) {
    this.real = real;
    this.imag = imag;
  }

  mul(other) {
    return new Complex(
      this.real * other.real - this.imag * other.imag,
      this.real * other.imag + this.imag * other.real,
    );
  }

  add(other) {
    return new Complex(
      this.real + other.real,
      this.imag + other.imag,
    );
  }

  abs() {
    return Math.sqrt(this.real * this.real + this.imag * this.imag);
  }
}
