import os,time,pathlib,subprocess,socket,struct,json
root=pathlib.Path('/tmp/ocenaudio-bench')
sock=max((root/'runtime').glob('sway-ipc*'),key=lambda p:p.stat().st_mtime)
def ipc(cmd,typ=0):
 with socket.socket(socket.AF_UNIX) as s:
  s.connect(str(sock));p=cmd.encode();s.sendall(b'i3-ipc'+struct.pack('<II',len(p),typ)+p);h=s.recv(14);n=struct.unpack('<II',h[6:])[0];d=b''
  while len(d)<n:d+=s.recv(n-len(d))
  return json.loads(d)
env=dict(os.environ,XDG_DATA_HOME=str(root/"data"),XDG_RUNTIME_DIR=str(root/'runtime'),XDG_CONFIG_HOME=str(root/'config'),XDG_STATE_HOME=str(root/'state'),XDG_CACHE_HOME=str(root/'cache'),DBUS_SESSION_BUS_ADDRESS=(root/'bus-address').read_text(),WAYLAND_DISPLAY=max((p for p in (root/'runtime').glob('wayland-*') if not p.name.endswith('.lock')),key=lambda p:p.stat().st_mtime).name,QT_QPA_PLATFORM='xcb',XDG_CURRENT_DESKTOP='sway')
def nodes(t):
 yield t
 for child in t.get('nodes',[])+t.get('floating_nodes',[]):yield from nodes(child)
def windows():return [n for n in nodes(ipc('',4)) if n.get('window')]
