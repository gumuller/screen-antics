#!/usr/bin/env python3
"""Review actual SDL playback on Hyprland; capture only the child player window."""
import argparse, subprocess, pathlib, time, json, math, re
from PIL import Image, ImageDraw
root=pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--scene', action='append', default=[], help='Scene name (repeatable); default all 41')
parser.add_argument('--out', type=pathlib.Path, default=pathlib.Path('captures/story-native-sweep'))
parser.add_argument('--aspect', default='16:9')
parser.add_argument('--resolution', default='1920x1080')
args=parser.parse_args()
out=root/args.out;out.mkdir(parents=True,exist_ok=True)
audit=json.loads((root/'evidence/hd2d-story-audit.json').read_text())
keys={
'SMDATE':[3.5,6.8,9.8,12.4,13.5,14.5,17.5,20,21.8,24.5,26.9,27.8,28.8,31.5],
'SUZYCITY':[1.5,5,10.8,13,14.8,16,19,24],
'SJLEAVES':[1,6,9.5,16,17.8,18.7,20.5,22],
'WOULDBE':[.8,7.5,9.5,10.8,12.5,14.5,20.8,22.7,24,25.5],
'MJSAND':[1,4,6,10.5,13.5,18,25,29,33,37.8],
'SBREAKUP':[3,6.5,8.5,11,14,18,23,26.5],
'SJGLIMPS':[2,4,10,15,18.5,21,26,30,33],
'SASKDATE':[3,13,17,20,22,24,28,33,37,42],
'SJMSSGE':[1,3,4.6,5.8,11,14,17,19,21,23.4,25.5],
'MEANWHIL':[.3,2,7,9,11.6,13.2,14.9,16.7,18.5],
'SJWORK':[.3,2,7,9,11.6,13.2,14.9,16.7,18.5],
'THEEND':[1,2.6,3.5,5.5,7.5,9,11,12.5,14.8],
'MJFIRE':[2,5,9,13,17,21,27,33,37.5,40.5,44,49,53],
'GJCATCH2':[2,5,9,13,17,21,24,26,28],
'MJFISH':[2,4,7,10,13,17,21,25,29,32],
}
# Newest stories first, then every remaining browser entry.
priority=['SMDATE','SUZYCITY','SJLEAVES','WOULDBE','MJSAND','SBREAKUP','SJGLIMPS','SASKDATE','SJMSSGE','SJMSUZY','MEANWHIL','SJWORK','THEEND']
scenes=sorted(audit['scenes'],key=lambda s:priority.index(s['scene'][:-4]) if s['scene'][:-4] in priority else 100)
results=[]
for scene in scenes:
 if args.scene and scene['scene'].removesuffix('.TTM') not in [n.upper().removesuffix('.TTM') for n in args.scene]:continue
 name=scene['scene'][:-4];dur=scene['extended_ticks']/60
 ts=keys.get(name,[.6]+[dur*f for f in [.12,.26,.4,.55,.7,.85]]+[dur-.3])
 cmd=[str(root/'target/release/screen-antics'),'--style','hd2d','--scene',name,'--aspect',args.aspect,'--resolution',args.resolution,'--fullscreen','--seconds',str(math.ceil(dur)+3.)]
 proc=subprocess.Popen(cmd,cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,bufsize=1)
 lines=[]
 while True:
  line=proc.stdout.readline();lines.append(line)
  if 'Panoramas:' in line:break
  if not line:raise RuntimeError('Player failed before ready: '+''.join(lines))
 start=time.monotonic();shots=[]
 for t in ts:
  time.sleep(max(0,start+t-time.monotonic()))
  windows=json.loads(subprocess.check_output(['hyprctl','clients','-j']))
  own=next((w for w in windows if w['pid']==proc.pid),None)
  if not own:
   tail=proc.communicate(timeout=5)[0];(out/f'{name}-failed.log').write_text(''.join(lines)+tail)
   raise RuntimeError(f'No owned player window {name} pid={proc.pid}, exit={proc.returncode}; see {out}/{name}-failed.log')
  x,y=own['at'];w,h=own['size'];path=out/f'{name}-{t:05.1f}.jpg'
  subprocess.run(['grim','-l','0','-g',f'{x},{y} {w}x{h}',str(out/'owned-window.png')],check=True)
  im=Image.open(str(out/'owned-window.png')).convert('RGB');im.thumbnail((1280,720));im.save(path,quality=91)
  shots.append({'time':t,'path':str(path.relative_to(root)),'title':own['title']})
 tail=proc.communicate(timeout=15)[0];lines.append(tail);log=''.join(lines);(out/f'{name}.log').write_text(log)
 m=re.search(r'Presented (\d+) frames in ([\d.]+)s \(([\d.]+) FPS\)',log)
 record={'scene':scene['scene'],'exit':proc.returncode,'duration_ticks':scene['extended_ticks'],'fps':float(m[3]) if m else None,'presented_frames':int(m[1]) if m else None,'seconds':float(m[2]) if m else None,'log':str((out/f'{name}.log').relative_to(root)),'captures':shots}
 results.append(record);(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
 sheet=Image.new('RGB',(1600,248*math.ceil(len(shots)/4)),(18,22,25));draw=ImageDraw.Draw(sheet)
 for i,s in enumerate(shots):
  im=Image.open(root/s['path']);im.thumbnail((400,225));x=i%4*400;y=i//4*248;sheet.paste(im,(x,y));draw.text((x+4,y+228),f"{name} {s['time']:.1f}s",fill='white')
 sheet.save(out/f'{name}-review.jpg',quality=91)
 print(f"{name}: {record['fps']} FPS, {len(shots)} owned-window captures, exit {proc.returncode}",flush=True)
print(f'{len(results)} native scenes finished.',flush=True)
