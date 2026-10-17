"""Read-only audit/contact sheets; no production artwork is modified.
Run audit_sprites.rs first to decode original resource frames into the audit directory.
"""
from pathlib import Path
import csv,json,struct,math,collections
import statistics
from PIL import Image,ImageDraw,ImageFont
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'evidence/sprite-review';ORIGINAL=OUT/'original';REMADE=ROOT/'assets/remastered/sprites'
FONT=ImageFont.truetype('/usr/share/fonts/TTF/DejaVuSans.ttf',12) if Path('/usr/share/fonts/TTF/DejaVuSans.ttf').exists() else ImageFont.load_default()
def load(path):
 b=path.read_bytes();assert b[:4]==b'SAHD';w,h=struct.unpack('<II',b[4:12]);assert len(b)==12+w*h*4;return Image.frombytes('RGBA',(w,h),b[12:])
class Mask(set):
 def __init__(self,pixels,width):super().__init__(pixels);self.width=width
 def sum(self):return len(self)
 def any(self):return bool(self)
def alpha(im):return Mask((i for i,p in enumerate(im.getchannel('A').tobytes())if p>127),im.width)
def bbox(mask):
 if not mask:return None
 xs=[p%mask.width for p in mask];ys=[p//mask.width for p in mask];return [min(xs),min(ys),max(xs)+1,max(ys)+1]
def iou(a,b):
 union=len(a|b);return len(a&b)/union if union else 1.
def centroid(mask):return [sum(p%mask.width for p in mask)/len(mask),sum(p//mask.width for p in mask)/len(mask)] if mask else None
def color_data(im):
 rgb=[(r,g,b)for r,g,b,a in im.getdata()if a>127];mean=[sum(p[c]for p in rgb)/len(rgb)for c in range(3)]if rgb else [0,0,0];return rgb,mean
def paste_thumb(canvas,im,box):
 x,y,w,h=box;thumb=im.copy();thumb.thumbnail((w,h),Image.Resampling.NEAREST);canvas.alpha_composite(thumb,(x+(w-thumb.width)//2,y+(h-thumb.height)//2))
rows=list(csv.DictReader((ORIGINAL/'frames.tsv').open(),delimiter='\t'));allframes=[];pairs=[];by_sheet=collections.defaultdict(list)
for row in rows:
 row['index']=int(row['index']);row['width']=int(row['width']);row['height']=int(row['height']);im=load(ORIGINAL/row['file']);mask=alpha(im);row.update(opaque_pixels=int(mask.sum()),opaque_bbox=bbox(mask),is_empty=not mask.any());allframes.append(row);by_sheet[row['sheet']].append((row,im))
for sheet,frames in by_sheet.items():
 for row,original in frames:
  target=REMADE/sheet/f"{row['index']:03}.rgba"
  if not target.exists():continue
  gen=load(target);gen_small=gen.resize(original.size,Image.Resampling.BOX);a=alpha(original);b=alpha(gen_small);ca=centroid(a);cb=centroid(b)
  scores=[]
  for candidate,other in frames:
   other=other.resize(original.size,Image.Resampling.NEAREST);scores.append((iou(alpha(other),b),candidate['index']))
  scores.sort(reverse=True);own=iou(a,b);best=scores[0]
  rgb,mean=color_data(gen);org_rgb,orgmean=color_data(original)
  p=dict(sheet=sheet,index=row['index'],original_size=list(original.size),replacement_size=list(gen.size),dimensions_exact=gen.size==(original.width*4,original.height*4),original_bbox=bbox(a),replacement_bbox_native=bbox(b),original_opaque_pixels=int(a.sum()),replacement_opaque_pixels_native=int(b.sum()),shape_iou=own,best_shape_match_index=best[1],best_shape_match_iou=best[0],shape_match_margin=best[0]-own,centroid_displacement_native=math.dist(ca,cb) if ca and cb else None,original_mean_rgb=orgmean,replacement_mean_rgb=mean,mean_rgb_displacement=math.dist(orgmean,mean),unique_replacement_rgb=len(set(rgb)) if len(rgb) else 0,original_empty=not a.any(),replacement_empty=not b.any())
  p['flags']=[]
  if own<.45:p['flags'].append('major silhouette mismatch')
  elif own<.65:p['flags'].append('silhouette mismatch')
  if best[1]!=row['index'] and best[0]-own>.10:p['flags'].append('another original pose matches silhouette better')
  if p['original_empty']!=p['replacement_empty']:p['flags'].append('empty-frame mismatch')
  pairs.append(p)
# All original frames, ordered by archive sheet name and frame index.
for page,start in enumerate(range(0,len(allframes),80)):
 canvas=Image.new('RGBA',(1600,1400),(27,30,34,255));d=ImageDraw.Draw(canvas);d.text((12,8),f'ORIGINAL ARTWORK - page {page+1} - indexed source frames, transparent pixels dark gray',font=FONT,fill='white')
 for k,row in enumerate(allframes[start:start+80]):
  x=k%10*160;y=30+k//10*170;d.rectangle((x,y,x+158,y+168),outline=(65,65,65));d.text((x+4,y+3),row['sheet'],font=FONT,fill='white');d.text((x+4,y+19),f"#{row['index']:03} {row['width']}x{row['height']}",font=FONT,fill=(190,190,190));im=load(ORIGINAL/row['file']);paste_thumb(canvas,im.resize((im.width*2,im.height*2),Image.Resampling.NEAREST),(x+4,y+37,150,125))
 canvas.convert('RGB').save(OUT/f'original-page-{page+1:02}.jpg',quality=94)
# All 96 replacement frames, paired with originals at identical common registration.
for sheet,frames in by_sheet.items():
 sp=[p for p in pairs if p['sheet']==sheet]
 for page,start in enumerate(range(0,len(sp),16)):
  canvas=Image.new('RGBA',(1600,1160),(27,30,34,255));d=ImageDraw.Draw(canvas);d.text((12,8),f'{sheet} - original / generated - page {page+1}; IoU measures alpha shape, not semantic pose correctness',font=FONT,fill='white')
  for k,p in enumerate(sp[start:start+16]):
   x=k%4*400;y=30+k//4*280;index=p['index'];original=frames[index][1];gen=load(REMADE/sheet/f'{index:03}.rgba');scale=min(175/original.width,210/original.height,4);w=max(1,round(original.width*scale));h=max(1,round(original.height*scale))
   d.rectangle((x,y,x+397,y+278),outline=(75,75,75));d.text((x+6,y+3),f"#{index:03} IoU {p['shape_iou']:.2f}; best #{p['best_shape_match_index']:03}",font=FONT,fill='white');d.text((x+8,y+20),'ORIGINAL',font=FONT,fill=(180,180,180));d.text((x+205,y+20),'GENERATED',font=FONT,fill=(180,180,180));canvas.alpha_composite(original.resize((w,h),Image.Resampling.NEAREST),(x+10,y+42));canvas.alpha_composite(gen.resize((w,h),Image.Resampling.LANCZOS),(x+205,y+42));d.text((x+6,y+260),'; '.join(p['flags'])[:60],font=FONT,fill=(255,150,100))
  canvas.convert('RGB').save(OUT/f'paired-{sheet}-page-{page+1:02}.jpg',quality=96)
summary={'source':'Native Rust decode of original/RESOURCE.001; SAHD replacement packs','original_sheets':len(by_sheet),'original_frames':len(allframes),'original_empty_frames':sum(f['is_empty'] for f in allframes),'replacement_frames':len(pairs),'replacement_sheets':len({p['sheet']for p in pairs}),'shape_iou_mean':statistics.mean(p['shape_iou']for p in pairs),'shape_iou_median':statistics.median(p['shape_iou']for p in pairs),'frames_below_065_iou':sum(p['shape_iou']<.65 for p in pairs),'frames_below_045_iou':sum(p['shape_iou']<.45 for p in pairs),'another_pose_better_by_010':sum(p['shape_match_margin']>.10 for p in pairs),'empty_mismatches':sum(p['original_empty']!=p['replacement_empty']for p in pairs),'dimension_mismatches':sum(not p['dimensions_exact']for p in pairs),'limitations':'IoU and nearest silhouette are screening metrics only. Different drawn proportions, alpha coverage and original anatomical exaggeration can change shape despite correct pose. Semantic pose correctness requires reviewing paired sheets.'}
(OUT/'manifest.json').write_text(json.dumps({'summary':summary,'original_frames':allframes,'replacement_audit':pairs},indent=2));(OUT/'summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary,indent=2))
with (OUT/'replacement-audit.csv').open('w')as f:
 fields=['sheet','index','shape_iou','best_shape_match_index','shape_match_margin','centroid_displacement_native','mean_rgb_displacement','unique_replacement_rgb','flags'];writer=csv.DictWriter(f,fieldnames=fields,extrasaction='ignore');writer.writeheader();writer.writerows(pairs)
