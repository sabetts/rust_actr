"""Python port of ACT-R:tutorial;unit5;declarative-issues.lisp."""

import rust_actr as actr

m = actr.Model('declarative-issues')

m.sgp('seed', [100, 1])
m.sgp('esc', True)
m.sgp('v', True)
m.sgp('bll', 0.5)
m.sgp('ans', 0.25)
m.sgp('mas', 2)
m.sgp('mp', 10)

m.chunk_type('simple-value', 'result')
m.chunk_type('simple-fact', 'item', 'attribute')
m.chunk_type('task-state', 'state')
m.chunk_type('number', 'visual-rep')
m.chunk_type('math-fact', 'arg1', 'arg2', 'result', 'operator')
m.chunk_type('context', 'val1', 'val2', 'val3', 'goal')

m.define_chunks(actr.Chunk('r2', 'chunk', {}))
m.define_chunks(actr.Chunk('r3', 'chunk', {}))
m.define_chunks(actr.Chunk('r4', 'chunk', {}))
m.define_chunks(actr.Chunk('r5', 'chunk', {}))
m.define_chunks(actr.Chunk('r6', 'chunk', {}))
m.define_chunks(actr.Chunk('r7', 'chunk', {}))
m.define_chunks(actr.Chunk('r8', 'chunk', {}))
m.add_chunk('zero', 'number', visual_rep=actr.String('0'))
m.add_chunk('one', 'number', visual_rep=actr.String('1'))
m.add_chunk('two', 'number', visual_rep=actr.String('2'))
m.add_chunk('three', 'number', visual_rep=actr.String('3'))
m.add_chunk('add', 'chunk')
m.add_chunk('subtract', 'chunk')
m.add_chunk('sky', 'chunk')
m.add_chunk('grass', 'chunk')
m.add_chunk('rose', 'chunk')
m.add_chunk('1-1=0', 'math-fact', arg1='one', arg2='one', result='zero', operator='subtract')
m.add_chunk('2-1=1', 'math-fact', arg1='two', arg2='one', result='one', operator='subtract')
m.add_chunk('3-2=1', 'math-fact', arg1='three', arg2='two', result='one', operator='subtract')
m.add_chunk('1+1=2', 'math-fact', arg1='one', arg2='one', result='two', operator='add')
m.add_chunk('1+2=3', 'math-fact', arg1='one', arg2='two', result='three', operator='add')
m.add_chunk('v1', 'simple-value', result=actr.String('true'))
m.add_chunk('v2', 'simple-value', result=actr.String('false'))
m.add_chunk('v3', 'simple-value', result=None)
m.add_chunk('f1', 'simple-fact', item='sky', attribute='blue')
m.add_chunk('f2', 'simple-fact', item='rose', attribute='red')
m.add_chunk('f3', 'simple-fact', item='grass', attribute='green')
m.add_chunk('g1', 'task-state', state='r8')
m.add_chunk('old-context', 'context', val1='one', val2='two', val3='add', goal='g1')

m.set_base_levels([('v1', 1, -1500), ('v2', 1, -1500), ('v3', 1, -1500)])
m.declare_buffer_usage('imaginal', 'context', all_slots=True)

m.add_production(
    '''
    (p p1
       ?goal>
         buffer empty
     ==>
       +goal>
         isa task-state
         state r2
       +retrieval>
         isa simple-value
        - result nil)
    '''
)

m.add_production(
    '''
    (p p2
       =goal>
         isa task-state
         state r2
       =retrieval>
     ==>
       =goal>
         state r3
       +retrieval>
         isa simple-value)
    '''
)

m.add_production(
    '''
    (p p3
       =goal>
         isa task-state
         state r3
       =retrieval>
         isa simple-value
     ==>
       =goal>
         state r4
       +retrieval>
         isa simple-fact
         attribute pink)
    '''
)

m.add_production(
    '''
    (p p4
       =goal>
         isa task-state
         state r4
       ?retrieval>
         state free
     ==>
       =goal>
         state r5
       +retrieval>
         isa simple-fact
         attribute black)
    '''
)

m.add_production(
    '''
    (p p5
       =goal>
         isa task-state
         state r5
       ?imaginal>
         state free
       =retrieval>
     ==>
       =goal>
         state r6
       +imaginal>
         isa context
         val1 one
         val2 two
       +retrieval>
         isa math-fact
         - arg1 nil
         - arg2 nil
         - result nil)
    '''
)

m.add_production(
    '''
    (p p6
       =goal>
         isa task-state
         state r6
       =imaginal>
         isa context
     ==>
       +retrieval>
         isa math-fact
         - arg1 nil
         - arg2 nil
         - result nil
       =goal>
         state r7
       =imaginal>
         val3 add)
    '''
)

m.add_production(
    '''
    (p p7
       =goal>
         isa task-state
         state r7
       =imaginal>
       =retrieval>
     ==>
       =goal>
         state r8
       =imaginal>
         goal =goal)
    '''
)

m.add_production(
    '''
    (p p8
       =goal>
         isa task-state
         state r8
     ==>
       +goal>
       -imaginal>)
    '''
)


if __name__ == "__main__":
    print(m.run(10.0), end="")
