"""Technical alpha isolation and registration of generated sprite atlas cells.

Human poses retain their largest connected alpha silhouette so neighboring atlas
fragments never influence frame bounds. Original multipart object/effect cells
retain all components. This does not repaint artwork.
"""
from pathlib import Path
from collections import deque
import csv,json,struct
from PIL import Image,ImageFilter
ROOT=Path(__file__).resolve().parents[1]
ASSETS=ROOT/'assets/remastered'
REGISTRATION=ROOT/'evidence/sprite-registration/frames.tsv'

def isolate_main(image):
    w,h=image.size; alpha=image.getchannel('A'); values=alpha.tobytes();seen=bytearray(w*h);components=[]
    for seed,a in enumerate(values):
        if a<=32 or seen[seed]:continue
        seen[seed]=1;component=[];queue=deque([seed])
        while queue:
            p=queue.popleft();component.append(p);x=p%w;y=p//w
            for ny in range(max(0,y-1),min(h,y+2)):
                for nx in range(max(0,x-1),min(w,x+2)):
                    n=ny*w+nx
                    if values[n]>32 and not seen[n]:seen[n]=1;queue.append(n)
        components.append(component)
    if not components:return image,0
    keep=max(components,key=len);mask=bytearray(w*h)
    for p in keep:mask[p]=255
    # Preserve faint anti-aliased boundary pixels adjoining the retained silhouette.
    expanded=Image.frombytes('L',(w,h),bytes(mask)).filter(ImageFilter.MaxFilter(3)).tobytes()
    cleaned=bytearray(values)
    for p,a in enumerate(values):
        if not mask[p] and not (a<=32 and expanded[p]):cleaned[p]=0
    result=image.copy();result.putalpha(Image.frombytes('L',(w,h),bytes(cleaned)))
    return result,len(components)-1

all_rows=list(csv.DictReader(REGISTRATION.open(),delimiter='\t'))
for name,atlas_name,nrows in [('JOHNWALK.BMP','johnwalk-remade-atlas.png',4),('MJJOG2.BMP','MJJOG2.BMP.remade-atlas.png',3),('MJREAD.BMP','MJREAD.BMP.remade-atlas.png',4)]:
    atlas=Image.open(ASSETS/atlas_name);dest=ASSETS/'sprites'/name;dest.mkdir(exist_ok=True)
    manifest_path=dest/'manifest.json'
    manifest=json.loads(manifest_path.read_text()) if manifest_path.exists() else {'atlas':atlas_name,'kind':'imagegen-redrawn','dimensions':list(atlas.size),'columns':10,'rows':nrows,'frames':[]}
    manifest['frames']=[];removed=0
    for row in [r for r in all_rows if r['sheet']==name]:
        r={k:int(v) for k,v in row.items() if k!='sheet'};i=r['index'];w,h=r['width']*4,r['height']*4
        box=(round(i%10*atlas.width/10),round(i//10*atlas.height/nrows),round((i%10+1)*atlas.width/10),round((i//10+1)*atlas.height/nrows))
        # Isolate the tiny star explicitly because the neighboring generated ripple overlaps this cell.
        if name=='MJREAD.BMP' and i==35:box=(1143,608,1174,637)
        human=name!='MJREAD.BMP' or i<20 or i in [21,22,23,26,27,28,29,30]
        # Generated figures sometimes extend slightly beyond their nominal cells.
        # Expand the technical ROI, then select the main silhouette, to avoid
        # clipping hands, noses and feet at an advisory generated grid boundary.
        if human:
            dx=round(atlas.width/10*0.15);dy=round(atlas.height/nrows*0.10)
            box=(max(0,box[0]-dx),max(0,box[1]-dy),min(atlas.width,box[2]+dx),min(atlas.height,box[3]+dy))
        cell=atlas.crop(box)
        count=0
        if human or (name=='MJREAD.BMP' and i in [20,24,25]):cell,count=isolate_main(cell);removed+=count
        bbox=cell.getchannel('A').point(lambda a:255 if a>96 else 0).getbbox();dst=Image.new('RGBA',(w,h))
        if bbox and r['right']>r['left'] and r['bottom']>r['top']:
            crop=cell.crop(bbox).resize(((r['right']-r['left'])*4,(r['bottom']-r['top'])*4),Image.Resampling.LANCZOS)
            dst.paste(crop,(r['left']*4,r['top']*4))
        (dest/f'{i:03}.rgba').write_bytes(b'SAHD'+struct.pack('<II',w,h)+dst.tobytes())
        if (dest/f'{i:03}.png').exists():dst.save(dest/f'{i:03}.png')
        manifest['frames'].append({'index':i,'width':w,'height':h,'atlas_cell':box,'source_bounds':[r[k]for k in ['left','top','right','bottom']],'isolation':'largest connected silhouette' if human else 'original multipart object/effect','discarded_components':count})
    manifest_path.write_text(json.dumps(manifest,indent=2));print(name,'discarded',removed,'neighbor/noise components')
