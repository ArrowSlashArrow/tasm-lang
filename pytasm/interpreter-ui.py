# prints example of interpreter ui
# used as a refernce durign interpreter development

from rich import pretty, traceback as tb, console as cns
console = cns.Console()

def ceil(n: float):
    return int(n) if n.is_integer() else int(n) + 1

MEMSIZE = 8
MEMREG = 41141
PTRPOS = 4
MEMMODE = 1
MEMORY = [  # dummy values
    12380,
    1,
    0,
    0,
    239480,
    45,
    67,
    -12391293
]

TIME = 240

COUNTERS = {
    "C91": 1209303,
    "C11": 235235,
    "C92": 88,
    "C9998": 0
}

WIDTH, HEIGHT = 136, 70

# all must be length 1
CORNER = "+"
HORIZONTAL = "-"
VERTICAL = "|"

ROWS = 40

def main():
    out_str = "" #"\x1b[2J\x1b[H"
    
    if MEMSIZE > 0:
        memcell_text_width = len(str(MEMSIZE - 1))

        memcell_width = 16 + memcell_text_width
        columns = min(ceil(MEMSIZE / ROWS), WIDTH // memcell_width)
        
        left = ceil((memcell_text_width) / 2 + 3)
        right = ceil((memcell_text_width + 1) / 2 + 3)
        first_column = CORNER + HORIZONTAL * left + " MEMORY " + HORIZONTAL * right + CORNER
        next_column = HORIZONTAL * (memcell_width - 1) + CORNER
        
        lines = []
        for i in range(MEMSIZE):
            if i >= ROWS:
                lines[i % ROWS].append(i)
            else:
                lines.append([i])
        
        def build_memcell_str(i):
            if i != PTRPOS:
                index_str = f" {str(i):0>{memcell_text_width}}: "
                return index_str + ("-" if MEMORY[i] < 0 else " ") + f"{str(abs(MEMORY[i])):0>10} {VERTICAL}"
            else:
                return " " * memcell_text_width + " > " + ("-" if MEMORY[i] < 0 else " ") + f"{str(abs(MEMORY[i])):0>10} {VERTICAL}"
                
        # top
        out_str += first_column + next_column * (columns - 1) + "\n"
        
        # memory cells
        for i, line in enumerate(lines):
            column = "".join([build_memcell_str(memcell) for memcell in line])
            if i == MEMSIZE % ROWS and MEMSIZE % ROWS != 0:
                out_str += (VERTICAL + column)[:-1] + CORNER + next_column + "\n"
            else:
                out_str += VERTICAL + column + "\n"
                
        # bottom row
        bottom_row = CORNER + next_column * (MEMSIZE // ROWS)
        if len(bottom_row) < 25:
            bottom_row += HORIZONTAL * (24 - len(bottom_row)) + CORNER
        else:
            bottom_row = bottom_row[:24] + "+" + bottom_row[25:]
        
        if columns == 1:
            bottom_row = bottom_row[:17] + "+" + bottom_row[18:]
        
        mode_str = "?????"
        if MEMMODE == 1:
            mode_str = " READ"
        elif MEMMODE == 2:
            mode_str = "WRITE"        
        
        out_str += bottom_row + "\n"
        out_str += f"{VERTICAL} Register: " + ("-" if MEMREG < 0 else " ") + f"{str(abs(MEMREG)):0>10} {VERTICAL}\n"
        out_str += f"{VERTICAL} Pointer:   " + f"{str(PTRPOS):0>10} {VERTICAL}\n"
        out_str += f"{VERTICAL} Pointer mode:   {mode_str} {VERTICAL}\n" 
        out_str += f"+-----------------------+\n\n"
        
    if len(COUNTERS) > 0:
        
        left_len = 5
        right_len_int = max([len(str(int(c))) for c in list(COUNTERS.values())])
        float_lengths = [min(len(str(c % 1)), 2) for s, c in COUNTERS.items() if not s[0] == "C"]
        right_len_float = max(float_lengths) if len(float_lengths) > 0 else -1 # -1 = no dp
        
        length = 6 + left_len + right_len_int + right_len_float

        out_str += f"{CORNER}{" COUNTERS ":{HORIZONTAL}^{length}}{CORNER}\n"
        
        right_padding = any([counter[0] == "T" for counter in list(COUNTERS.keys())])
        for counter, value in COUNTERS.items():
            out_str += f"{VERTICAL} {counter:<{left_len}} {VERTICAL} {str(int(value)):>{right_len_int}}"
            if right_len_float > -1 and counter[0] == "T":
                out_str += f".{str(int(value * 100) % 100):0>2} {VERTICAL}\n"
            else:
                out_str += f"{"   " if right_padding else ""} {VERTICAL}\n"
        
        out_str += f"{CORNER}{"-" * length}{CORNER}\n\n"
        
    
    out_str += f"Time: {TIME / 240:.3f}s"
        
    print(out_str)

try:
    if __name__ == "__main__":
        main()
except:
    console.print(tb.Traceback())