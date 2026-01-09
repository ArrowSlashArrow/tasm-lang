from tasm_parser import *
from deserialiser import *
from serialiser import *
from parser import *
from rich import traceback as tb, console as cns
import sys, time, tasm_parser, pygetwindow, gdobj, json, subprocess, signal, win32gui

console = cns.Console()
argv = sys.argv
# command syntax: python main.py <input_file> <output_level>

# all of the tags here were wrong.
# k4: level tag
# k2: level name
# k5: author

# object id reference table:
# 3619: item edit trigger
# 3620: item compare
# 3641: persistent item
# 1615: counter
# 1268: spawn trigger

# this program was made by </> in the span of a few days
# it's my first assembler :)
args = [
    "main.py",
    "<input_file>",
    "<options>"
]
options = {
    "-h": "Print help message",
    "--append": "append new objs to existing objects?",
    "--no-warn": "disable warning messages?",
    "--no-routine-text": "exclude routine text markers from level?",
    "--show-namespace": "[dbg] print instructions of each routine?",
    "--slow": "option to disable squishing of objects in the actual level. helpful for debugging",
    "--superfast": "option to run the interpreter as fast as possible",
    "--no-write": "disable writing output to save file?",
    "--group-offset <offset>": "start at group <offset>. default is 0",
    "--coll-block-offset <offset>": "start collision blocks at <offset>. default is 0",
    # "--counter-offset <offset>": "start counter ids at <offset>. default is 0",
    "--mem-ptr-pos <offset>": "mem ptr conter id is <offset>. this is the last id used by the memory block. default is 9999",
    "--read-only": "only read the level contents?",
    "--disable-bit-packing": "disables bit packing for large numbers when compiling.",
    "--index <index>": "write to <index>th level in savefile. default is 0.",
    "--interpret": "Tells the compiler to simulate the gd engine. Does not compile to a level, but is helpful for debugging"
}

def get_obj_md_from_str(object):
    print(object)
    obj = gdobj.GDObject({})
    params = [i for i in object.split(",")]

    # chop off the unnecessary semicolon
    if params[-1][-1] == ";":
        params[-1] = params[-1][:-1]

    for i in range(0, len(params), 2):
        obj.add_param(params[i], params[i + 1])

    print(obj)
    obj.print_raw_params()

def display_help_text():
    print(f"\nCommand syntax: python {" ".join(args)}\n")
    print("Available options")
    for option, desc in options.items():
        print(f"{option}: {desc}")
    print()
    
def main():
    # help text
    if "-h" in argv or "--help" in argv or len(argv) == 1:
        display_help_text()
        return
    
    # parse argv
    file = os.path.relpath(argv[1])
    append = "--append" in argv
    no_warnings = "--no-warn" in argv
    level_dbg_text = "--routine-text" not in argv
    fast = "--slow" not in argv
    superfast = "--superfast" in argv
    read_only = "--read-only" in argv
    bit_packing = "--disable-bit-packing"
    interpret = "--interpret" in argv
    display_namespace = "--show-namespace" in argv
    nowrite = "--no-write" in argv
    
    level_index = 0
    group_offset = 0
    coll_block_offset = 0
    counter_offset = 0
    for arg_idx, arg in enumerate(argv):
        if arg == "--index":
            try:
                level_index = int(argv[arg_idx + 1])
            except:
                print("Invalid index supplied.")
                return
        if arg == "--group-offset":
            try:
                group_offset = int(argv[arg_idx + 1])
            except:
                print("Invalid group offset supplied.")
                return
        if arg == "--coll-block-offset":
            try:
                coll_block_offset = int(argv[arg_idx + 1])
            except:
                print("Invalid collision block offset supplied.")
                return
        # if arg == "--counter-offset":
        #     try:
        #         counter_offset = int(argv[arg_idx + 1])
        #     except:
        #         print("Invalid counter id offset supplied.")
        #         return
        if arg == "--mem-ptr-pos":
            try:
                gdobj.PTRPOS = int(argv[arg_idx + 1])
                if gdobj.PTRPOS > 9999:
                    raise ValueError
                gdobj.update_memreg_id()
                tasm_parser.update_aliases()
            except:
                print("Invalid mem ptr pos specified (int, max = 9999).")
                return
    
    
    # verify that the input file exists if writing
    if not os.path.exists(file) and not read_only:
        print(f"Could not find file {file}. First argument is the input file.")
        return
    
    # we dont need gd data if we are not writing to it
    if not interpret:
        # wait for gd window to be closed
        count = 1
        
        while True:
            result = pygetwindow.getWindowsWithTitle("Geometry Dash")
            if not any(result):
                break
            # show the title of the window (because sometimes it isn't GD)
            windows_str = ", ".join([win32gui.GetWindowText(r._hWnd) for r in result])
            print(f"\x1b[KPlease close {windows_str}." + "." * (count % 3), end="\r")
            time.sleep(1 / 3)
            count += 1
            
        del count
        print()

        levels_root = get_local_levels().getroot()
        level_xml = levels_root.find("dict").findall("d")[0].findall("d")[level_index]
        
        # get references to data
        level = list(level_xml)
        level_data_idx = -1
        level_name_idx = -1
        raw_level_str = ""
        for index, tag in enumerate(level):
            if tag.text == "k4":
                level_data_idx = index + 1
                break
            elif tag.text == "k2":
                level_name_idx = index + 1

        if level_data_idx < 0:
            print("Could not find level. Please open the level in the editor, and close it again.")
            return
        
        # decrypt level
        raw_level_str = level[index + 1].text
        try:
            data = decrypt(raw_level_str).decode()
        except:
            print("Could not decrypt level.")
            return
        
        # parse the raw level str
        header, *objs, _ = data.split(";")
        old_objs = ";".join(objs)

        if read_only:
            for obj in objs:
                get_obj_md_from_str(obj)    
            return
    
    # parse inthe input program
    if not bit_packing:
        gdobj.bit_packing_enabled = False
    print(f"Parsing {file}...")
    routines = parse_tasm(file, no_warnings)

    errors = tasm_parser.errors
    if errors > 0:
        print(f"Could not parse {file} because of {errors} errors.")
        return
    
    namespace = determine_groups(routines, display_namespace)

    # optional namespace display
    if display_namespace:
        for group, name in enumerate(list(namespace.keys())):
            print(f"group {group}: routine {name}")

        if start_group > 0:
            print(f"main group: {start_group}")

    if interpret:
        print("Running interpreter...")
        json.dump({"routines": namespace}, open("namespace.json", "w"), indent=4)
        try:
            executable = "interpreter\\target\\debug\\interpreter.exe" if "--runner" in argv else "interpreter.exe"
            process = subprocess.Popen([executable, "namespace.json", "--fast" if superfast else ""])  # compiled rust program
            process.wait()
        except KeyboardInterrupt:
            process.send_signal(signal.SIGINT)
            process.wait()
        return
    
    # put the program back into the level
    print(f"Serialising objects to '{os.path.basename(file)}'...")

    
    
    # get the new objects as a string
    new_objs = parse_namespace(
        namespace, 
        group_offset,
        coll_block_offset,
        counter_offset,
        level_dbg_text, 
        fast, 
        not no_warnings
    )
    
    errors = tasm_parser.errors
    if errors > 0:
        print(f"Could not parse {file} because of {errors} errors.")
        return
    
    if append:  # overwrites level if not appending
        new_objs = old_objs + str(new_objs)
    
    if not nowrite:
        print("Encrypting level...")
        # concat new objs to data
        combined_data = header + new_objs
        encrypted = encrypt_level_string(combined_data)

        level[level_data_idx].text = encrypted.decode()
        level[level_name_idx].text = os.path.basename(file)
        
        os.remove(decoded_path)
        print("Encrypting savefile...")
        xml_str = '<?xml version="1.0"?>' + ET.tostring(levels_root, encoding="unicode")

        encrypted_savefile = encrypt_savefile_str(xml_str.encode("utf-8"))
        open(local_levels, "wb").write(encrypted_savefile)
    else:
        try:
            os.remove(decoded_path)
        except:
            pass
try:
    if __name__ == "__main__":
        main()
except KeyboardInterrupt:
    exit()
except:
    console.print(tb.Traceback())
