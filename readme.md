# Banklog converter
This is an experimental tool for converting the PJBoy's Super Metroid banklogs into an asar-assemblable format that can be reassembled to an original SM ROM.

You can find an interactive web-based viewer for the bank logs here: [Bank logs](http://patrickjohnston.org/bank/index.html)

# Running
- Run "download_banks.py" in the "logs" folder to download the latest bank logs.
- Run "cargo run --release" to start the conversion (run in release mode so it doesn't take ages)
- Hopefully you'll have output in the "asm" folder that you can now assemble with asar, using the "main.asm" file as the starting point.

# Configuring
In the config folder there are two sub-folders where YAML files can be placed.
- labels - These files will be read and parsed as labels to be used in the conversion.
- overrides - These files will modify and flag code and data that the automatic conversion can't handle

# WIP
Still very much work-in-progress. It can output valid output, but labels and more are still very experimental.