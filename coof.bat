start "mpv" mpv --input-ipc-server=\\.\pipe\tmp\mpv-socket --scripts=.\coof.js --video-sync=display-resample --pause=no https://stream.wotos.eu/snw_med.m3u8
timeout /T 2
coof http://stream.wotos.eu/spreader \\.\pipe\tmp\mpv-socket
