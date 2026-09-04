import json, os, time, urllib.request, urllib.parse
UA = {'User-Agent':'mtg-imsaho-pixelart/1.0','Accept':'*/*'}
CARDS = [
 # (name, set, color-bucket)
 ("Geist of Saint Traft","isd","W"), ("Doomed Traveler","isd","W"),
 ("Chapel Geist","isd","W"), ("Elite Inquisitor","isd","W"),
 ("Midnight Haunting","isd","W"),
 ("Snapcaster Mage","isd","U"), ("Delver of Secrets","isd","U"),
 ("Invisible Stalker","isd","U"), ("Laboratory Maniac","isd","U"),
 ("Stitched Drake","isd","U"),
 ("Liliana of the Veil","isd","B"), ("Diregraf Ghoul","isd","B"),
 ("Unburial Rites","isd","B"), ("Bloodline Keeper","isd","B"),
 ("Grimgrin, Corpse-Born","isd","B"),
 ("Brimstone Volley","isd","R"), ("Devil's Play","isd","R"),
 ("Balefire Dragon","isd","R"), ("Instigator Gang","isd","R"),
 ("Blasphemous Act","isd","R"),
 ("Mayor of Avabruck","isd","G"), ("Kessig Cagebreakers","isd","G"),
 ("Garruk Relentless","isd","G"), ("Spider Spawning","isd","G"),
 ("Gatstaf Shepherd","isd","G"),
 ("Plains","isd","L"), ("Island","isd","L"), ("Swamp","isd","L"),
 ("Mountain","isd","L"), ("Forest","isd","L"),
 ("Kessig Wolf Run","isd","L"), ("Blazing Torch","isd","A"),
]
def get(url):
    return urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=30).read()
meta = {}
for name, st, bucket in CARDS:
    q = f"https://api.scryfall.com/cards/named?exact={urllib.parse.quote(name)}&set={st}"
    try:
        d = json.loads(get(q))
    except Exception as e:
        print("META FAIL", name, e); continue
    iu = d.get('image_uris') or (d.get('card_faces',[{}])[0].get('image_uris',{}))
    meta[name] = dict(bucket=bucket, type_line=d.get('type_line',''),
                      mana_cost=d.get('mana_cost',''), artist=d.get('artist',''),
                      oracle=d.get('oracle_text',''), art=iu.get('art_crop'),
                      normal=iu.get('normal'), cn=d.get('collector_number'))
    slug = name.lower().replace(' ','_').replace(',','').replace("'",'')
    for kind in ('art','normal'):
        u = meta[name][kind]
        if not u: continue
        p = f"refs/{slug}_{kind}.jpg"
        if not os.path.exists(p):
            open(p,'wb').write(get(u)); time.sleep(0.12)
    print(f"ok {name:28} {meta[name]['type_line'][:30]:30} {meta[name]['artist']}")
    time.sleep(0.1)
json.dump(meta, open('refs/meta.json','w'), indent=1)
print(len(meta), "cards fetched")
