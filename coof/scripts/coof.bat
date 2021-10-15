start "mpv" mpv --input-ipc-server=\\.\pipe\tmp\mpv-socket --scripts=.\coof.js --video-sync=display-resample --pause=no https://stream.wotos.eu/snw_master.m3u8
timeout /T 2
coof --server http://stream.wotos.eu/spreader --mpv-socket \\.\pipe\tmp\mpv-socket
