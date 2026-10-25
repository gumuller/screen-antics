#!/usr/bin/env python3
"""Extract new HD-2D semantic clips from generated transparent masters.

This never reads original game sprites. Alpha-component extraction tolerates the
nonuniform grid spacing of generated sheets, preserving pixels and head scale.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT=Path(__file__).resolve().parents[1]

def components(image):
    w,h=image.size;alpha=image.getchannel('A').tobytes();runs=[];previous=[];parents=[]
    def root(i):
        while parents[i]!=i:
            parents[i]=parents[parents[i]];i=parents[i]
        return i
    for y in range(h):
        current=[];x=0;prior=0
        while x<w:
            if alpha[y*w+x]<64:x+=1;continue
            start=x
            while x<w and alpha[y*w+x]>=64:x+=1
            end=x;idx=len(runs);parents.append(idx);runs.append((y,start,end));current.append(idx)
            while prior<len(previous) and runs[previous[prior]][2]<start:prior+=1
            scan=prior
            while scan<len(previous) and runs[previous[scan]][1]<=end:
                a,b=root(idx),root(previous[scan])
                if a!=b:parents[a]=b
                scan+=1
        previous=current
    groups={}
    for i,(y,x1,x2) in enumerate(runs):
        g=groups.setdefault(root(i),{'area':0,'sx':0,'sy':0,'box':[w,h,0,0],'runs':[]})
        n=x2-x1;g['area']+=n;g['sx']+=(x1+x2-1)*n/2;g['sy']+=y*n;g['runs'].append((y,x1,x2))
        b=g['box'];b[0]=min(b[0],x1);b[1]=min(b[1],y);b[2]=max(b[2],x2);b[3]=max(b[3],y+1)
    for g in groups.values():g['center']=(g['sx']/g['area'],g['sy']/g['area'])
    return list(groups.values())

def extract(master,cols,rows,paired=False):
    groups=components(master)
    if paired:
        merged=[{'area':0,'runs':[],'box':[master.width,master.height,0,0]} for _ in range(cols*rows)]
        for g in groups:
            x,y=g['center'];col=min(cols-1,int(x/master.width*cols));row=min(rows-1,int(y/master.height*rows))
            target=merged[row*cols+col];target['area']+=g['area'];target['runs'].extend(g['runs']);b=target['box'];q=g['box']
            target['box']=[min(b[0],q[0]),min(b[1],q[1]),max(b[2],q[2]),max(b[3],q[3])]
        groups=merged
        for g in groups:
            b=g['box'];g['center']=((b[0]+b[2])/2,(b[1]+b[3])/2)
    major=[g for g in groups if g['area']>=400]
    # Small detached embers/eye highlights belong to the nearest full sprite.
    if len(major)>cols*rows:
        major=sorted(major,key=lambda g:g['area'],reverse=True)[:cols*rows]
    if len(major)!=cols*rows:raise ValueError(f'Expected {cols*rows} silhouettes, found {len(major)}')
    buckets=[[] for _ in range(rows)]
    for g in major:
        r=min(range(rows),key=lambda r:abs(g['center'][1]-(r+.5)*master.height/rows))
        buckets[r].append(g)
    if any(len(row)!=cols for row in buckets):
        # Uneven generated row spacing: rank centers vertically, then horizontally.
        ordered=sorted(major,key=lambda g:g['center'][1]);buckets=[ordered[r*cols:(r+1)*cols] for r in range(rows)]
    ordered=[g for row in buckets for g in sorted(row,key=lambda g:g['center'][0])]
    for g in groups:
        if any(g is m for m in major):continue
        target=min(ordered,key=lambda m:(m['center'][0]-g['center'][0])**2+(m['center'][1]-g['center'][1])**2)
        if sum((a-b)**2 for a,b in zip(target['center'],g['center']))<180**2:
            target['runs'].extend(g['runs']);b=target['box'];q=g['box'];target['box']=[min(b[0],q[0]),min(b[1],q[1]),max(b[2],q[2]),max(b[3],q[3])]
    output=[]
    for g in ordered:
        x1,y1,x2,y2=g['box'];x1=max(0,x1-2);y1=max(0,y1-2);x2=min(master.width,x2+2);y2=min(master.height,y2+2)
        crop=master.crop((x1,y1,x2,y2));mask=Image.new('L',crop.size,0);draw=ImageDraw.Draw(mask)
        for y,a,b in g['runs']:draw.line((a-x1,y-y1,b-1-x1,y-y1),fill=255)
        mask=mask.filter(ImageFilter.MaxFilter(5));crop.putalpha(ImageChops.multiply(crop.getchannel('A'),mask))
        output.append((crop,[x1,y1,x2-x1,y2-y1]))
    return output

# frame ranges are source cell ordinals; all output is newly generated artwork.
SHEETS={
 'johnny-movement-v1':(6,4,[('johnny-walk',range(0,6),4.,'feet'),('johnny-run',range(6,12),4.,'feet'),('johnny-climb',range(12,18),4.,'feet'),('johnny-swim',range(18,24),4.,'center')]),
 'johnny-actions-v1':(6,4,[('johnny-idle',range(0,6),4.,'feet'),('johnny-read',range(6,12),4.,'feet'),('johnny-drink',range(12,15),4.,'feet'),('johnny-fish',range(15,18),4.,'feet'),('johnny-sleep',[18],4.,'feet'),('johnny-wake',[19],4.,'feet'),('johnny-help',[20,21],4.,'feet'),('johnny-fall',[22,23],4.,'feet')]),
 'props-fire-v1':(6,4,[(name,[i],None,'feet') for i,name in enumerate(['coconut','coconut-open','bottle','book','fish-tropical','fish-silver','fishing-rod','shovel','bucket','sandcastle','flowers','gift','raft','rowboat','life-ring','driftwood','letter','telescope'])]+[('campfire',range(18,24),None,'feet')]),
 'fauna-v1':(6,4,[('gull',range(0,6),None,'center'),('shark',range(6,12),None,'center'),('octopus',range(12,16),None,'center'),('fish-silver',[16],None,'center'),('fish-tropical',[17],None,'center'),('crab',[18,19],None,'feet'),('parrot',[20,21],None,'feet'),('dolphin',[22,23],None,'center')]),
 'vehicles-v1':(4,3,[(name,[i],None,'feet') for i,name in enumerate(['galleon','galleon-furled','liner','tanker','biplane','biplane-up','motorboat','rowboat','raft-sail','raft','sandcastle','telescope'])]),
 'cast-v1':(6,4,[('suzy-read',[0],4.,'feet'),('suzy-sit',[1],4.,'feet'),('suzy-wave',[2],4.,'feet'),('suzy-walk',range(3,6),4.,'feet'),('mermaid',range(6,12),4.,'center'),('johnny-tux',range(12,18),4.,'feet'),('neptune',[18,19],4.,'feet'),('sailor',range(20,24),8.,'feet')]),
}
SHEETS.update({
 'johnny-work-v1':(6,4,[('johnny-dig',range(0,6),4.,'feet'),('johnny-build',range(6,12),4.,'feet')]+[(name,[i],4.,'feet') for i,name in enumerate(['johnny-angry','johnny-fist','johnny-sad','johnny-dizzy','johnny-flower','johnny-hot','johnny-telescope','johnny-ignite','johnny-tend','johnny-reach','johnny-scared','johnny-row'],12)]),
 'story-props-v1':(4,3,[('helicopter',[0,1],None,'center')]+[(name,[i],None,'feet') for i,name in enumerate(['parachute','dinner-table','pumpkin','christmas-tree','clock','nest','thought-island','thought-clock','bananas','lantern'],2)]),
 'interactions-v1':(4,3,[(name,[i],5.,'feet') for i,name in enumerate(['suzy-offer','suzy-accept','suzy-embrace','suzy-kiss','mermaid-dance','mermaid-dance-turn','mermaid-embrace','mermaid-kiss','date-dinner','date-laugh','suzy-refuse','suzy-goodbye'])]),
})
SHEETS['johnny-actions-v1']=(6,4,[('johnny-idle',[0,1],4.,'feet'),('johnny-look',[3],4.,'feet'),('johnny-thinking',[4],4.,'feet'),('johnny-read',range(6,12),4.,'feet'),('johnny-drink',range(12,15),4.,'feet'),('johnny-fish',[15],4.,'feet'),('johnny-fish-tug',[16],4.,'feet'),('johnny-fish-catch',[17],4.,'feet'),('johnny-sleep',[18],4.,'feet'),('johnny-wake',[19],4.,'feet'),('johnny-help',[20,21],4.,'feet'),('johnny-fall',[22],4.,'feet'),('johnny-recover',[23],4.,'feet')])
SHEETS['cast-v1']=(6,4,[('suzy-read',[0],4.,'feet'),('suzy-sit',[1],4.,'feet'),('suzy-wave',[2],4.,'feet'),('suzy-walk',range(3,6),4.,'feet'),('mermaid',[6],4.,'feet'),('mermaid-wave',[7],4.,'feet'),('mermaid-swim',[8,9],4.,'center'),('mermaid-sit',[10],4.,'feet'),('mermaid-turn',[11],4.,'feet'),('johnny-tux',[12],4.,'feet'),('johnny-tux-walk',[13,14],4.,'feet'),('johnny-tux-kneel',[15],4.,'feet'),('johnny-tux-dance',[16],4.,'feet'),('johnny-tux-sit',[17],4.,'feet'),('neptune',[18],4.,'feet'),('neptune-surface',[19],4.,'feet'),('sailor',[20],8.,'feet'),('sailor-walk',[21,22],8.,'feet'),('sailor-row',[23],8.,'feet')])
SHEETS['office-props-v1']=(4,2,[('johnny-office',range(4),4.,'feet'),('gramophone',[4],None,'feet'),('splash',[5],None,'center'),('raft-building',[6],None,'feet'),('chair',[7],None,'feet')])
SHEETS['special-actions-v1']=(4,3,[('johnny-guitar',range(4),4.,'feet'),('johnny-bath',range(4,8),4.,'feet'),('johnny-dive-ready',[8],4.,'feet'),('johnny-dive-jump',[9],4.,'feet'),('johnny-dive',[10],4.,'center'),('johnny-dive-down',[11],4.,'center')])
SHEETS['gag-props-v1']=(4,3,[(name,[i],None,'feet') for i,name in enumerate(['guitar','wine-glass','fruit-plate','soup-bowl','roast-fish','oyster','idea-bulb','thought-question','music','seashell','map','spyglass'])])
SHEETS['motion-poses-v1']=(4,3,[(name,[i],4.,'center' if i==8 else 'feet') for i,name in enumerate(['johnny-rest','johnny-stretch-up','johnny-stretch-left','johnny-stretch-right','johnny-bend','johnny-rise','johnny-tired','johnny-wave','johnny-skydive','johnny-parachute','johnny-land','johnny-book-stand'])])
SHEETS['gait-cycles-v1']=(4,4,[('johnny-walk',range(8),4.,'feet'),('johnny-run',range(8,16),4.,'feet')])
SHEETS['gag-actions-v1']=(4,3,[(name,[i],5.,'feet') for i,name in enumerate(['johnny-coconut-hold','johnny-coconut-pick','johnny-coconut-wind','johnny-coconut-throw','johnny-coconut-drink','johnny-coconut-hit','johnny-coconut-dizzy','johnny-bird-sit','johnny-bird-swat','johnny-fish-pull'])]+[('johnny-octopus',range(10,12),5.,'feet')])
SHEETS['fishing-actions-v2']=(4,3,[('johnny-fishing-cast',range(8),4.,'feet'),('johnny-fishing-pickup',[8],4.,'feet'),('johnny-fishing-carry',range(9,12),4.,'feet')])
SHEETS['firecraft-v2']=(4,3,[('johnny-firecraft',range(9),6.,'feet'),('johnny-roast',range(9,12),6.,'feet')])
SHEETS['campfire-meal-v1']=(4,3,[('johnny-hot-meal',range(12),6.,'feet')])
SHEETS['fishing-gags-v1']=(4,3,[('johnny-fishing-gag',range(12),4.8,'feet')])
SHEETS['pole-gait-v1']=(4,4,[('johnny-pole-walk',range(8),4.,'feet'),('johnny-pole-run',range(8,16),4.,'feet')])
SHEETS['food-carry-v1']=(4,4,[('johnny-food-walk',range(8),5.,'feet'),('johnny-food-pickup',range(8,12),5.,'feet'),('fish-green',range(12,15),None,'center'),('bait-tin',[15],None,'feet')])
SHEETS['raft-build-v2']=(4,3,[('johnny-raft-build',range(12),5.,'feet')])
SHEETS['diving-judges-v1']=(3,4,[(name,range(i*3,i*3+3),12.,'feet') for i,name in enumerate(['judge-starfish','judge-crab','judge-octopus','judge-gull'])])
SHEETS['shark-gag-v1']=(4,3,[('johnny-shark-reaction',range(8),6.,'feet'),('shark-head',range(8,12),5.5,'feet')])
SHEETS['mask-dance-v1']=(4,4,[('johnny-mask-dance',range(8),4.8,'feet'),('johnny-mask-reaction',range(8,16),4.8,'feet')])
SHEETS['gulliver-gag-v1']=(4,3,[('johnny-gulliver',range(6),5.,'feet'),('tiny-sailor-walk',range(6,8),6.,'feet'),('tiny-sailor-work',range(8,10),6.,'feet'),('tiny-rowboat',range(10,12),8.,'feet')])
SHEETS['diving-actions-v2']=(4,3,[('johnny-dive-sequence',range(10),4.8,'center'),('johnny-water-tread',range(10,12),4.8,'feet')])
SHEETS['weather-gull-boat-v1']=(4,3,[('gull-perch',range(4),14.,'feet'),('storm-cloud',range(4,8),4.,'center'),('tiny-rowboat-empty',range(8,10),8.,'feet'),('wood-mask',[10],None,'feet'),('leaf-skirt',[11],None,'feet')])
SHEETS['heat-dressing-v1']=(4,3,[('johnny-heat',range(8),5.4,'feet'),('johnny-mask-dress',range(8,12),5.4,'feet')])
SHEETS['rescue-wave-v1']=(4,3,[('johnny-mask-wave',[0,1,2,1],5.4,'feet'),('johnny-leaf-wave',range(4,8),5.4,'feet'),('palm-fan',[8,9],None,'feet'),('rope-coil',[10,11],None,'feet')])
SHEETS['book-theft-v1']=(4,3,[('johnny-book-theft',range(7),5.7,'feet'),('reading-log',[7],5.7,'feet'),('gull-book',range(8,12),11.,'feet')])
SHEETS['coconut-actions-v2']=(4,3,[('johnny-tree-shake',[0,2],5.3,'feet'),('johnny-coconut-actions',range(4,12),5.3,'feet')])
SHEETS['coconut-props-v2']=(2,2,[('cracking-rock',[0],None,'feet'),('cracking-rock-halves',[1],None,'feet'),('coconut-whole',[2],None,'feet'),('coconut-half',[3],None,'feet')])
SHEETS['spyglass-actions-v1']=(4,2,[('johnny-spyglass',range(8),6.3,'feet')])
SHEETS['bath-theft-v1']=(4,3,[('johnny-bath-theft',range(7),8.2,'feet'),('clothes-stick',[7],7.5,'feet'),('empty-clothes-stick',[8],7.9,'feet'),('gull-shorts',range(9,12),14.,'feet')])
SHEETS['gull-reader-boat-v1']=(4,2,[('gull-reading',range(4),20.,'feet'),('clown-boat',range(4,8),4.,'feet')])
SHEETS['coconut-throw-v2']=(4,2,[('johnny-coconut-throwing',range(8),6.3,'feet')])
SHEETS['shark-gulp-v1']=(4,3,[('shark-gulp',range(8),4.,'feet'),('johnny-gulp-reaction',range(8,12),5.,'feet')])
SHEETS['fallen-fishing-pole-v1']=(1,1,[('fallen-fishing-pole',[0],None,'feet')])
SHEETS['plane-rescue-v1']=(4,2,[('rescue-biplane',range(4),5.2,'feet'),('pilot-fall',[4],11.,'feet'),('pilot-deploy',[5],10.,'feet'),('pilot-parachute',[6],8.,'feet'),('pilot-afloat',[7],13.5,'feet')])
SHEETS['sandcastle-kong-v1']=(4,3,[('johnny-sand-build',range(4),5.3,'feet'),('johnny-kong',range(4,8),5.3,'feet'),('sandcastle-stages',range(8,12),6.5,'feet')])
SHEETS['story-thoughts-v2']=(4,2,[('story-thought',range(8),3.6,'feet')])
SHEETS['office-actions-v2']=(2,2,[('johnny-office-seated',range(4),3.7,'feet')])
SHEETS['message-bottle-v1']=(4,3,[('johnny-message',range(12),5.7,'feet')])
SHEETS['mermaid-invitation-v1']=(4,3,[('mermaid-gift',range(4),5.7,'feet'),('johnny-invitation',range(4,10),5.7,'feet'),('sailor-cap',[10],None,'feet'),('flower-lei',[11],None,'feet')])
SHEETS['suzy-reunion-v1']=(4,3,[('suzy-beach-read',[0,1,2],5.7,'feet'),('suzy-beach-stand',[3],5.7,'feet'),('suzy-bottle-pick',[4],5.7,'feet'),('suzy-run',[6,7],5.7,'feet'),('suzy-reunion',range(8,12),5.7,'feet')])
SHEETS['suzy-message-v1']=(2,2,[('suzy-letter',[0],8.8,'feet'),('beach-towel',[1],10.5,'feet'),('suzy-thought',[2,3],5.8,'feet')])
SHEETS['raft-journey-v1']=(4,2,[('johnny-raft-journey',range(5),5.3,'feet'),('johnny-bundle',[5],5.3,'feet'),('johnny-raft-load',[6],5.3,'feet'),('journey-raft',[7],5.3,'feet')])
SHEETS['party-boat-v1']=(4,3,[('party-yacht',range(4),2.8,'feet'),('johnny-party',range(4,9),5.7,'feet'),('johnny-water-help',[9],6.5,'feet'),('waterskier',[10,11],6.,'feet')])
SHEETS['dialogue-actions-v1']=(4,2,[('johnny-dialogue',range(6),6.3,'feet'),('johnny-pole-glance',[6,7],6.3,'feet')])
SHEETS['date-actions-v1']=(4,2,[('johnny-date-setup',range(4),6.3,'feet'),('date-couple',range(4,8),6.3,'feet')])
SHEETS['dinner-table-v2']=(2,2,[('date-table',[0],8.5,'feet'),('date-gramophone',[1],8.5,'feet'),('date-seated',[2,3],8.5,'feet')])
SHEETS['date-mishap-v1']=(2,2,[('journey-raft-empty',[0],8.,'feet'),('johnny-tux-startled',[1],7.4,'feet'),('mermaid-dive',[2],8.4,'center'),('date-lift-mishap',[3],5.7,'feet')])
SHEETS['tanker-close-v1']=(1,1,[('tanker',[0],None,'feet')])
# Measured in the generated master image, never inferred from changing sprite
# bounds. Contacts make the line, tinder and body meet at the same world point.
FISH_FEET=[(142,365),(510,364),(921,364),(1242,365),(166,680),(551,678),(980,677),(1310,680),(156,981),(558,982),(927,981),(1287,982)]
FISH_TIPS=[(356,129),(646,15),(733,136),(1495,66),(452,494),(771,470),(1132,408),(1505,398),(408,925),(764,845),(1135,845),(1494,847)]
FIRE_CONTACTS=[(310,362),(635,358),(1024,358),(1424,358),(340,640),(696,640),(1098,648),(1460,648),(303,984)]
RAFT_CONTACTS=[(202,423),(505,426),(809,426),(1118,423),(200,812),(522,812),(815,816),(1125,816),(176,1196),(519,1196),(820,1202),(1120,1202)]
MEAL_CONTACTS=[(105,317),(500,317),(883,317),(1270,317),(98,636),(495,637),(945,637),(1270,637),(112,955),(510,955),(884,955),(1267,955)]
FOOD_FEET=[(147,1018),(487,1018),(776,1018),(1047,1018)]
SHARK_HIPS=[(227,443),(500,440),(790,440),(1118,443),(210,853),(500,855),(785,858),(1106,850)]
JUDGE_CARDS=[(235,284),(640,256),(1056,75),(252,574),(644,532),(1058,378),(256,870),(654,828),(1064,670),(265,1168),(652,1098),(1077,964)]
GULLIVER_CONTACTS=[(155,330),(470,330),(785,330),(1100,330),(155,750),(480,750)]
DIVE_CONTACTS=[(146,370),(404,241),(771,261),(1126,225),(142,631),(473,662),(770,675),(1108,648),(163,1047),(466,1047),(755,1162),(1082,1171)]
HEAT_CONTACTS=[(160,425),(452,425),(755,425),(1108,425),(140,790),(427,790),(790,790),(1090,790)]
WAVE_FEET=[(188,432),(556,432),(905,432),(1285,432),(188,853),(556,853),(905,853),(1285,853)]
BOOK_FEET=[(205,375),(555,375),(910,375),(1280,375),(210,790),(555,790),(910,790),(1265,780)]
COCONUT_CONTACTS=[(312,130),(1070,155),(165,740),(540,740),(920,740),(1325,740),(230,1040),(650,1040),(1020,1040),(1390,1040)]
WORLD_WIDTH={'gramophone':36,'splash':30,'raft-building':65,'chair':25,'coconut':7,'coconut-open':7,'bottle':9,'book':15,'fish-tropical':12,'fish-silver':10,'fishing-rod':42,'shovel':27,'bucket':14,'sandcastle':48,'flowers':14,'gift':12,'raft':75,'raft-sail':85,'rowboat':55,'life-ring':14,'driftwood':25,'letter':12,'telescope':30,'campfire':28,'gull':26,'shark':85,'octopus':50,'crab':14,'parrot':15,'dolphin':90,'galleon':150,'galleon-furled':150,'liner':170,'tanker':180,'biplane':90,'biplane-up':90,'motorboat':70,'helicopter':95,'parachute':55,'dinner-table':95,'pumpkin':14,'christmas-tree':50,'clock':15,'nest':20,'thought-island':70,'thought-clock':55,'bananas':12,'lantern':15,'guitar':32,'wine-glass':8,'fruit-plate':24,'soup-bowl':16,'roast-fish':24,'oyster':12,'idea-bulb':15,'thought-question':55,'music':22,'seashell':10,'map':25,'spyglass':22}
WORLD_WIDTH.update({'fish-green':11,'bait-tin':9})
WORLD_WIDTH.update({'wood-mask':12,'leaf-skirt':22})
WORLD_WIDTH.update({'palm-fan':17,'rope-coil':12})
WORLD_WIDTH.update({'cracking-rock':22.6,'cracking-rock-halves':35.,'coconut-whole':15.,'coconut-half':14.})
WORLD_WIDTH.update({'fallen-fishing-pole':60.})
WORLD_WIDTH.update({'sailor-cap':23,'flower-lei':15})

def write_image(image,target):
    target.parent.mkdir(parents=True,exist_ok=True)
    image=image.convert('RGBA');image.save(target)
    target.with_suffix('.rgba').write_bytes(b'SAHD'+struct.pack('<II',*image.size)+image.tobytes())

def crop_with_edges(image,box):
    # Mechanical padding for the 20px missing top row in the supplied Night/Ocean
    # master. Never paint black where the actual scene should continue.
    x,y,w,h=box
    canvas=image.crop((x,y,x+w,y+h))
    if y<0:
        band=image.crop((x,0,x+w,1)).resize((w,-y),Image.Resampling.NEAREST)
        canvas.paste(band,(0,0))
    return canvas

def prepare_backgrounds(pack):
    source=ROOT/'assets/hd2d'
    inputs=sorted(source.glob('backgrounds/*.png'))+sorted(source.glob('widescreen/*/*.png'))+[source/'widescreen/layouts.tsv']
    panoramas={
        'ISLAND2.SCR':'day','ISLETEMP.SCR':'day','NIGHT.SCR':'night',
        'OCEAN00.SCR':'ocean','OCEAN01.SCR':'ocean','OCEAN02.SCR':'ocean',
    }
    inputs+=sorted((pack/'masters').glob('panorama-*-v2.png'))
    stamp={'version':2,'inputs':[(str(p.relative_to(ROOT)),p.stat().st_mtime_ns,p.stat().st_size) for p in inputs]}
    stamp=json.loads(json.dumps(stamp));cache=pack/'background-stamp.json'
    if cache.exists() and json.loads(cache.read_text())==stamp and (pack/'scene-backgrounds.tsv').exists():
        files=[line.split('\t')[-1] for line in (pack/'scene-backgrounds.tsv').read_text().splitlines()[1:]]
        if all((pack/f).exists() for f in files):return
    rows=['background\tmode\twidth\theight\tx\ty\tstage_width\tstage_height\tfile']
    layouts={}
    for row in (source/'widescreen/layouts.tsv').read_text().splitlines()[1:]:
        f=row.split('\t');layouts.setdefault(f[0],[]).append(f)
    for path in sorted((source/'backgrounds').glob('*.png')):
        name=path.stem
        if name.endswith('.wide'):continue
        if name in panoramas:
            # One continuous generated master, mechanically cropped for each
            # camera. Shared stage pixels are identical in every aspect mode;
            # no mirrored or stretched edge strips are constructed at runtime.
            master=Image.open(pack/'masters'/f'panorama-{panoramas[name]}-v2.png').convert('RGBA')
            master=master.resize((3440,1440),Image.Resampling.LANCZOS)
            core=master.crop((440,20,3000,1420))
            for mode,cw,ch in [('core',2560,1400),('16_9',2560,1440),('21_9',3360,1440),('43_18',3440,1440)]:
                sx,sy=(cw-2560)//2,(ch-1400)//2
                x,y=(3440-cw)//2,(1440-ch)//2
                image=master.crop((x,y,x+cw,y+ch))
                assert image.crop((sx,sy,sx+2560,sy+1400)).tobytes()==core.tobytes(),(name,mode)
                relative=Path('backgrounds')/name/(mode+'.png');write_image(image,pack/relative)
                rows.append('\t'.join(map(str,[name,mode,cw,ch,sx,sy,2560,1400,relative.with_suffix('.rgba').as_posix()])))
            continue
        normalize=name=='NIGHT.SCR' or name.startswith('OCEAN')
        core=Image.open(path).convert('RGBA')
        if normalize:core=core.crop((0,0,2560,1400))
        relative=Path('backgrounds')/name/'core.png';write_image(core,pack/relative)
        rows.append('\t'.join(map(str,[name,'core',*core.size,0,0,*core.size,relative.with_suffix('.rgba').as_posix()])))
        for f in layouts[name]:
            ratio=f[1];wide=Image.open(source/'widescreen'/Path(f[8]).with_suffix('.png')).convert('RGBA')
            sx,sy,sw,sh=map(int,f[4:8])
            if normalize:
                bounds=wide.convert('RGB').getbbox()
                if not bounds:raise ValueError(f'{name}: no painted photo content')
                sx-=bounds[0];sy-=bounds[1];wide=wide.crop(bounds)
                a,b=map(int,ratio.split('_'));cw=max(2560,round(1400*a/b));ch=round(cw*b/a)
                nx=(cw-2560)//2;ny=(ch-1400)//2
                wide=crop_with_edges(wide,(sx-nx,sy-ny,cw,ch));sx,sy,sw,sh=nx,ny,2560,1400
            assert wide.crop((sx,sy,sx+sw,sy+sh)).tobytes()==core.tobytes(),(name,ratio)
            relative=Path('backgrounds')/name/(ratio+'.png');write_image(wide,pack/relative)
            rows.append('\t'.join(map(str,[name,ratio,*wide.size,sx,sy,sw,sh,relative.with_suffix('.rgba').as_posix()])))
    (pack/'scene-backgrounds.tsv').write_text('\n'.join(rows)+'\n')
    cache.write_text(json.dumps(stamp,indent=2)+'\n')

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--pack',type=Path,default=ROOT/'assets/hd2d-v2');args=parser.parse_args()
    prepare_backgrounds(args.pack)
    clean=args.pack/'masters/ending-clean-v1.png'
    if clean.exists():
        with Image.open(clean) as image:
            write_image(image.resize((2560,1400),Image.Resampling.LANCZOS),args.pack/'backdrops/ending-clean.png')
    result={'format':'screen-antics-semantic-hd2d-v1','clips':{},'sources':[]}
    preview=[];contacts=['clip\tindex\tname\tx\ty']
    for stem,(cols,rows,clips) in SHEETS.items():
        path=args.pack/'masters'/f'{stem}.png'
        if not path.exists():continue
        with Image.open(path) as image:
            if image.mode!='RGBA' or image.getchannel('A').getextrema()[0]!=0:raise ValueError(f'{path}: actual transparent alpha required')
            if stem=='pole-gait-v1':
                # Measured row boundaries: the generated running feet touch the
                # next row's hair, so connected-component extraction merges two
                # independent poses. Unpack the authored cells instead.
                xs=[0,410,795,1175,1536];ys=[0,300,572,816,1024];frames=[]
                for row in range(4):
                    for col in range(4):
                        x,y=xs[col],ys[row];cell=image.crop((x,y,xs[col+1],ys[row+1]));box=cell.getchannel('A').getbbox()
                        crop=cell.crop(box);frames.append((crop,[x+box[0],y+box[1],crop.width,crop.height]))
            else:
                frames=extract(image,cols,rows,paired=stem in ("interactions-v1","gag-props-v1","fishing-gags-v1","coconut-props-v2","gull-reader-boat-v1","story-thoughts-v2","suzy-message-v1"))
        result['sources'].append({'file':str(path.relative_to(args.pack)),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'frames':len(frames)})
        for name,indices,ppu,anchor in clips:
            picked=[frames[i] for i in indices]
            if ppu is None:ppu=max(im.width for im,_ in picked)/WORLD_WIDTH[name]
            if stem=='motion-poses-v1':ppu=4.8
            if name=='johnny-dive':ppu=5.2
            if name=='johnny-dive-down':ppu=4.8
            entries=[];target=args.pack/'clips'/name;target.mkdir(parents=True,exist_ok=True)
            for old in target.iterdir():
                if old.suffix in ('.png','.rgba') and old.stem.isdigit() and int(old.stem)>=len(picked):old.unlink()
            for index,(im,rect) in enumerate(picked):
                png=target/f'{index:03}.png';im.save(png)
                png.with_suffix('.rgba').write_bytes(b'SAHD'+struct.pack('<II',*im.size)+im.tobytes())
                entries.append({'file':str(png.with_suffix('.rgba').relative_to(args.pack)),'width':im.width,'height':im.height,'source_rect':rect,'anchor':[im.width*.5,im.height if anchor=='feet' else im.height*.5],'flip_x':(name=='gull' and index in (0,5)) or (stem=='interactions-v1' and name.startswith('suzy-') and name!='suzy-offer')})
                if stem=='fishing-actions-v2':
                    source_index=list(indices)[index];foot=FISH_FEET[source_index];tip=FISH_TIPS[source_index]
                    entries[-1]['anchor']=[foot[0]-rect[0],foot[1]-rect[1]]
                    contacts.append('\t'.join(map(str,[name,index,'rod_tip',tip[0]-rect[0],tip[1]-rect[1]])))
                if name=='johnny-firecraft':
                    contact=FIRE_CONTACTS[index];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                    contacts.append('\t'.join(map(str,[name,index,'tinder',*entries[-1]['anchor']])))
                if name=='johnny-hot-meal':
                    # Register the seated actor by the hips, not the extended
                    # legs, held fish or changing width of the flame plume.
                    contact=MEAL_CONTACTS[index];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                if name=='johnny-raft-build':
                    contact=RAFT_CONTACTS[index];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                    contacts.append('\t'.join(map(str,[name,index,'raft',*entries[-1]['anchor']])))
                if name=='johnny-food-pickup':
                    contact=FOOD_FEET[index];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                if name=='johnny-shark-reaction':
                    contact=SHARK_HIPS[index];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]];entries[-1]['flip_x']=index==6
                if name.startswith('judge-'):
                    contact=JUDGE_CARDS[list(indices)[index]]
                    contacts.append('\t'.join(map(str,[name,index,'card_center',contact[0]-rect[0],contact[1]-rect[1]])))
                if name=='johnny-gulliver':
                    contact=GULLIVER_CONTACTS[index];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                if stem=='diving-actions-v2':
                    contact=DIVE_CONTACTS[list(indices)[index]];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                if name=='johnny-heat':
                    contact=HEAT_CONTACTS[index];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                if name in ('johnny-mask-wave','johnny-leaf-wave'):
                    contact=WAVE_FEET[list(indices)[index]];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                if name in ('johnny-book-theft','reading-log'):
                    contact=BOOK_FEET[list(indices)[index]];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                if name in ('johnny-tree-shake','johnny-coconut-actions'):
                    contact=COCONUT_CONTACTS[index+(0 if name=='johnny-tree-shake' else 2)];entries[-1]['anchor']=[contact[0]-rect[0],contact[1]-rect[1]]
                    contacts.append('\t'.join(map(str,[name,index,'palm' if name=='johnny-tree-shake' else 'rock' if index>=3 else 'feet',*entries[-1]['anchor']])))
                if name=='cracking-rock-halves':
                    entries[-1]['anchor']=[1163-rect[0],520-rect[1]]
                if name=='johnny-spyglass':
                    feet=[(200,464),(615,464),(995,464),(1465,464),(200,918),(625,918),(1050,918),(1465,918)][index]
                    entries[-1]['anchor']=[feet[0]-rect[0],feet[1]-rect[1]]
                if name=='johnny-bath-theft':
                    waist=[(174,365),(548,365),(895,365),(1242,365),(172,699),(549,699),(909,699)][index]
                    entries[-1]['anchor']=[waist[0]-rect[0],waist[1]-rect[1]]
                if name in ('clothes-stick','empty-clothes-stick'):
                    root=(1255,703) if name=='clothes-stick' else (170,1040)
                    entries[-1]['anchor']=[root[0]-rect[0],root[1]-rect[1]]
                if name=='johnny-coconut-throwing':
                    feet=[(185,479),(590,479),(978,479),(1330,479),(195,972),(562,972),(970,972),(1350,972)][index]
                    entries[-1]['anchor']=[feet[0]-rect[0],feet[1]-rect[1]]
                if name=='rescue-biplane':
                    center=[(223,230),(661,230),(1104,230),(1538,230)][index]
                    entries[-1]['anchor']=[center[0]-rect[0],center[1]-rect[1]]
                if name=='johnny-sand-build':
                    root=[(265,390),(625,390),(995,390),(1355,390)][index]
                    entries[-1]['anchor']=[root[0]-rect[0],root[1]-rect[1]]
                if name=='johnny-kong':
                    foot=[(174,766),(550,766),(915,766),(1260,766)][index]
                    entries[-1]['anchor']=[foot[0]-rect[0],foot[1]-rect[1]]
                if name=='johnny-office-seated':
                    wheel=[(230,717),(720,717),(230,1440),(720,1440)][index]
                    entries[-1]['anchor']=[wheel[0]-rect[0],wheel[1]-rect[1]]
                if name=='johnny-message':
                    foot=[(235,385),(575,385),(925,385),(1290,385),(195,750),(565,750),(925,750),(1285,750),(215,1065),(585,1065),(1000,1065),(1320,1065)][index]
                    entries[-1]['anchor']=[foot[0]-rect[0],foot[1]-rect[1]]
                    entries[-1]['flip_x']=index==9
                if name=='johnny-invitation':
                    foot=[(245,720),(560,720),(908,720),(1265,720),(210,1065),(570,1065)][index]
                    entries[-1]['anchor']=[foot[0]-rect[0],foot[1]-rect[1]]
                if name=='mermaid-gift':
                    root=[(210,345),(565,345),(922,345),(1285,345)][index]
                    entries[-1]['anchor']=[root[0]-rect[0],root[1]-rect[1]]
                if stem=='raft-journey-v1':
                    root=[(310,448),(700,448),(1095,448),(1560,448),(260,860),(645,860),(1140,860),(1555,860)][list(indices)[index]]
                    entries[-1]['anchor']=[root[0]-rect[0],root[1]-rect[1]]
                if stem=='dialogue-actions-v1':
                    source_index=list(indices)[index]
                    foot=[(205,430),(660,430),(1135,430),(1580,430),(240,855),(660,855),(1100,855),(1580,855)][source_index]
                    entries[-1]['anchor']=[foot[0]-rect[0],foot[1]-rect[1]]
                    if source_index>=6:
                        tip=[(1411,466),(1715,433)][source_index-6]
                        contacts.append('\t'.join(map(str,[name,index,'rod_tip',tip[0]-rect[0],tip[1]-rect[1]])))
                if name=='johnny-date-setup':
                    root=[(180,435),(700,435),(1130,435),(1655,435)][index]
                    entries[-1]['anchor']=[root[0]-rect[0],root[1]-rect[1]]
                if stem=='dinner-table-v2':
                    root=[(425,447),(1151,450),(394,955),(1123,955)][list(indices)[index]]
                    entries[-1]['anchor']=[root[0]-rect[0],root[1]-rect[1]]
                if name=='date-lift-mishap':
                    entries[-1]['anchor']=[1080-rect[0],1010-rect[1]]
                if name=='johnny-fishing-gag':
                    a=im.getchannel('A');data=list(a.crop((0,im.height-12,im.width,im.height)).tobytes());total=sum(data)
                    if total:entries[-1]['anchor'][0]=sum((j%im.width+.5)*v for j,v in enumerate(data))/total
            if name=='johnny-climb':
                for entry,(im,_) in zip(entries,picked):
                    lower=im.getchannel('A').crop((0,max(0,im.height-8),im.width,im.height));data=list(lower.tobytes())
                    total=sum(data)
                    if total:entry['anchor'][0]=sum((i%im.width+.5)*a for i,a in enumerate(data))/total
            if name in ('johnny-walk','johnny-run','johnny-pole-walk','johnny-pole-run','johnny-food-walk'):
                for index,(entry,(im,_)) in enumerate(zip(entries,picked)):
                    # Register locomotion to the pelvis, not the changing bounds
                    # of outstretched hands/feet. Pixel content stays untouched.
                    y0=int(im.height*.48);y1=int(im.height*.60)
                    band=im.getchannel('A').crop((0,y0,im.width,y1));data=list(band.tobytes());total=sum(data)
                    if total:entry['anchor'][0]=sum((i%im.width+.5)*a for i,a in enumerate(data))/total
                    if name in ('johnny-run','johnny-pole-run') and index in (3,7):entry['anchor'][1]+=16
            fps=4 if name in ('johnny-idle','johnny-read','johnny-help','johnny-octopus') else 3 if name=='johnny-drink' else 6 if name=='johnny-climb' else 8 if 'walk' in name else 10
            result['clips'][name]={'pixels_per_unit':ppu,'fps':fps,'frames':entries}
            preview.append((name,picked[0][0]))
    (args.pack/'scene-art.json').write_text(json.dumps(result,indent=2)+'\n')
    rows=['clip\tindex\tfps\tpixels_per_unit\tanchor_x\tanchor_y\tflip_x\tfile']
    for name,clip in result['clips'].items():
        for index,frame in enumerate(clip['frames']):
            rows.append('\t'.join(map(str,[name,index,clip['fps'],clip['pixels_per_unit'],*frame['anchor'],int(frame['flip_x']),frame['file']])))
    (args.pack/'scene-art.tsv').write_text('\n'.join(rows)+'\n')
    (args.pack/'scene-contacts.tsv').write_text('\n'.join(contacts)+'\n')
    w=1000;h=((len(preview)+4)//5)*180;canvas=Image.new('RGB',(w,h),(28,42,55));draw=ImageDraw.Draw(canvas)
    for i,(name,im) in enumerate(preview):
        image=im.copy();image.thumbnail((190,150));x=(i%5)*200;y=(i//5)*180
        canvas.paste(image,(x+(200-image.width)//2,y+25),image);draw.text((x+6,y+5),name,fill='white')
    canvas.save(args.pack/'clip-preview.png')
    print(json.dumps({'clips':len(result['clips']),'frames':sum(len(c['frames']) for c in result['clips'].values())}))
if __name__=='__main__':main()
