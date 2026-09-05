import sys
sys.dont_write_bytecode = True
import sys; sys.path.insert(0,'.')
import board
# A real mid-game position: turn 7, green-white werewolves vs blue-black zombies.
STATE = {
 'phase': 'T7 COMBAT',
 'opp': {'name':'GRISELDA','life':11,'hand':4,'lib':21,
   'lands':[{'name':'Swamp','tapped':True},{'name':'Swamp'},{'name':'Island','tapped':True},
            {'name':'Island'},{'name':'Nephalia'} if False else {'name':'Swamp','tapped':True}],
   'creatures':[{'name':'Diregraf Ghoul','pt':'2/2','tapped':True},
                {'name':'Stitched Drake','pt':'3/4'},
                {'name':'Grimgrin, Corpse-Born','pt':'5/5','dmg':2},
                {'name':'Bloodline Keeper','pt':'3/3','sick':True}]},
 'you': {'name':'YOU','life':16,'hand':4,'lib':19,
   'lands':[{'name':'Forest','tapped':True},{'name':'Forest'},{'name':'Plains','tapped':True},
            {'name':'Plains','tapped':True},{'name':'Kessig Wolf Run'}],
   'creatures':[{'name':'Gatstaf Shepherd','pt':'2/2'},
                {'name':'Doomed Traveler','pt':'1/1','tapped':True},
                {'name':'Garruk Relentless','pt':'3','dmg':1},
                {'name':'Elite Inquisitor','pt':'3/2','sick':True}],
   'hand':[{'name':'Brimstone Volley'},{'name':'Spider Spawning'},
           {'name':'Midnight Haunting'},{'name':'Blazing Torch'},
           {'name':'Chapel Geist'}]}}
if __name__ == '__main__':
    board.render_board(STATE, sys.argv[1] if len(sys.argv)>1 else 'board.png', scale=3)
