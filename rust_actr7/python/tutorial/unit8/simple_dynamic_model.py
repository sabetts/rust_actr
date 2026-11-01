"""Python port of ACT-R:tutorial;unit8;simple-dynamic-model.lisp."""

import rust_actr as actr

m = actr.Model('dynamic-introduction')

m.sgp(':v', True)
m.sgp(':trace-detail', 'high')
m.sgp(':cst', True)
m.sgp(':style-warnings', None)

m.chunk_type('fact', 'context', 'data')
m.chunk_type('step', 'step', 'destination')
m.chunk_type('result', 'data1', 'data2')

m.add_chunk('first', 'chunk')
m.add_chunk('second', 'chunk')
m.add_chunk('destination', 'chunk')
m.add_chunk('data2', 'chunk')
m.add_chunk('data3', 'chunk')
m.add_chunk('data', 'chunk')
m.add_chunk('a', 'step', step='first', destination='data2')
m.add_chunk('b', 'step', step='second', destination='data3')

m.goal_focus(isa='fact', context='data', data=10)

m.add_production(
    '''
    (p start
       =goal>
         isa      fact
         context  =context
         =context =x
       ?imaginal>
         state    free
         buffer   empty
      ==>
       =goal>
         context  destination
       +imaginal>
         isa      result
         data1    =x
       +retrieval>
         isa      step
         step     first)
    '''
)

m.add_production(
    '''
    (p retrieve-first-step
       =goal>
         isa     fact
         context =slot
         data    =x
       =imaginal>
         isa     result
         data1   =x
       =retrieval>
         isa     step
         =slot   =target
         step    first
      ==>
       =goal>
         data    11
       =imaginal>
         =target =x
       +retrieval>
         isa     step
         step    second)
    '''
)

m.add_production(
    '''
    (p retrieve-second-step
       =goal>
         isa     fact
         context =slot
         data    =x
       =imaginal>
       =retrieval>
         isa     step
         =slot   =target
         step    second
      ==>
       =imaginal>
         =target =x)
    '''
)


if __name__ == "__main__":
    print(m.run(10.0), end="")
