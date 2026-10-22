import struct, sys
R=4
cells=[(x,y) for y in range(-R,R+1) for x in range(-R,R+1) if abs(x+y)<=R]
idx={c:i for i,c in enumerate(cells)}
def rot(c): x,y=c; z=-x-y; return (-z,-x)        # 60° rotation in axial coords
def refl(c): x,y=c; return (y,x)
maps=[]
for r in (False,True):
    for k in range(6):
        perm=[]
        for c in cells:
            d=refl(c) if r else c
            for _ in range(k): d=rot(d)
            perm.append(idx[d])
        maps.append(perm)
data=open(sys.argv[1],'rb').read()
sols=set()
for i in range(0,len(data),8):
    lo,hi=struct.unpack_from('<II',data,i); sols.add(lo|(hi<<31))
missing=0
for s in sols:
    for p in maps[1:]:
        t=0
        for i in range(61):
            if s>>i&1: t|=1<<p[i]
        if t not in sols: missing+=1
print("solutions",len(sols),"symmetry images missing",missing)
