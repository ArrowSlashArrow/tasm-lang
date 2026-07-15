# this script was used to generate the bulk of routines for stdlib mem_8bit and mem_14bit.

bit = """\
bit_N:
    ifg bit_N_on, bit_M, _std_mem_temp, E    ; check bit N

bit_N_on:
    sub _std_mem_temp, E
    spawn bit_M | ordered:false delay:0.0 remap: {D}
"""

def generate_bit(n):
    m = n - 1
    e = 2 ** m
    keys = range(1, e + 1)
    values = range(e + 1, 2 * e + 1)
    d = ", ".join([f"{a}={b}" for a, b in zip(keys, values)])
    return bit.replace("M", str(m)).replace("N", str(n)).replace("D", d).replace("E", str(e))

for i in range(8, 1, -1):
    open("bits.txt", "a").write(generate_bit(i))