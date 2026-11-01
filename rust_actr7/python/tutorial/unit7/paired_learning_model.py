"""Python port of ACT-R:tutorial;unit7;paired-learning-model.lisp."""

import rust_actr as actr

m = actr.Model('paired-learning')

m.sgp(':esc', True)
m.sgp(':v', None)
m.sgp(':pct', True)
m.sgp(':ul', True)
m.sgp(':epl', True)
m.sgp(':rt', -1.7)
m.sgp(':lf', 0.2)
m.sgp(':ans', 0.4)
m.sgp(':egs', 0.1)
m.sgp(':bll', 0.5)
m.sgp(':trace-detail', 'low')
m.sgp(':iu', 5)
m.sgp(':alpha', 0.2)
m.sgp(':ncnar', None)

m.chunk_type('operator', 'pre', 'action', 'arg1', 'arg2', 'post')
m.chunk_type('task', 'state', 'step')
m.chunk_type('args', 'arg1', 'arg2')

m.define_chunks('ready')
m.define_chunks('start')
m.define_chunks('stimulus-read')
m.define_chunks('create')
m.define_chunks('read')
m.define_chunks('recalled')
m.define_chunks('fill')
m.define_chunks('filled')
m.define_chunks('associate')
m.define_chunks('wait')
m.define_chunks('respond')
m.define_chunks('test-arg2')
m.define_chunks('response')
m.define_chunks('type')
m.define_chunks('new-trial')
m.define_chunks('complete-task')
m.define_chunks('retrieving-operator')
m.define_chunks('attending')
m.define_chunks('retrieving-response')
m.add_chunk('op1', 'operator', pre='start', action='read', arg1='create', post='stimulus-read')
m.add_chunk('op2', 'operator', pre='stimulus-read', action='associate', arg1='filled', arg2='fill', post='recalled')
m.add_chunk('op3', 'operator', pre='recalled', action='test-arg2', arg1='respond', arg2='wait')
m.add_chunk('op4', 'operator', pre='respond', action='type', arg2='response', post='wait')
m.add_chunk('op5', 'operator', pre='wait', action='read', arg2='fill', post='new-trial')
m.add_chunk('op6', 'operator', pre='new-trial', action='complete-task', post='start')

m.set_all_base_levels(1000)

m.add_production(
    '''
    (p retrieve-operator
       =goal>
         isa    task
         state  =state
         step   ready
      ==>
       +retrieval>
         isa    operator
         pre    =state
       =goal>
         step   retrieving-operator)
    '''
)

m.add_production(
    '''
    (p read-arg1
       =goal>
         isa        task
         step       retrieving-operator
       =retrieval>
         isa        operator
         action     read
         arg1       create
         post       =state
       =visual-location>
       ?visual>
         state      free
       ?imaginal>
         state      free
      ==>
       +imaginal>
         isa        args
         arg1       fill
       +visual>
         cmd        move-attention
         screen-pos =visual-location
       =goal>
         step       attending
         state      =state)
    '''
)

m.add_production(
    '''
    (p encode-arg1
       =goal>
         isa     task
         step    attending
       =visual>
         isa     text
         value   =val
       =imaginal>
         isa     args
         arg1    fill
       ?imaginal>
         state   free
      ==>
       *imaginal>
         arg1    =val
       =goal>
         step    ready)
    '''
)

m.add_production(
    '''
    (p retireve-associate
       =goal>
         isa     task
         step    retrieving-operator
       =imaginal>
         isa     args
         arg1    =stimulus
       =retrieval>
         isa     operator
         action  associate
         arg1    filled
         arg2    fill
         post    =state
       ?imaginal>
         state   free
      ==>
       +retrieval>
         isa     args
         arg1    =stimulus
       *imaginal>
         arg2    fill
       =goal>
         step    retrieving-response
         state   =state)
    '''
)

m.add_production(
    '''
    (p retrieval-arg2-unsuccessful
       =goal>
         isa     task
         step    retrieving-response
       =imaginal>
         isa     args
         arg2    fill
       ?retrieval>
         buffer   failure
       ?imaginal>
         state  free
      ==>
       *imaginal>
         arg2   nil
       =goal>
         step   ready)
    '''
)

m.add_production(
    '''
    (p retrieval-arg2-successful
       =goal>
         isa    task
         step   retrieving-response
       =imaginal>
         isa    args
         arg2   fill
       =retrieval>
         isa    args
         arg2   =val
       ?imaginal>
         state  free
      ==>
       *imaginal>
         arg2   =val
       =goal>
         step   ready)
    '''
)

m.add_production(
    '''
    (p respond-to-retrieval-failure
       =goal>
         isa    task
         step   retrieving-operator
       =imaginal>
         isa    args
         arg2   nil
       =retrieval>
         isa    operator
         action test-arg2
         arg2   =state
       ?imaginal>
         state  free
      ==>
       =imaginal>
       =goal>
         state  =state
         step   ready)
    '''
)

m.add_production(
    '''
    (p respond-to-retrieval-success
       =goal>
         isa     task
         step    retrieving-operator
       =imaginal>
         isa     args
         arg2    =val
       =retrieval>
         isa     operator
         action  test-arg2
         arg1    =state
       ?imaginal>
         state   free
      ==>
       =imaginal>
       =goal>
         state   =state
         step    ready)
    '''
)

m.add_production(
    '''
    (p type-arg2
       =goal>
         isa     task
         step    retrieving-operator
       =imaginal>
         isa     args
         arg2    =val
       =retrieval>
         isa     operator
         action  type
         arg2    response
         post    =state
       ?imaginal>
         state   free
       ?manual>
         state   free
      ==>
       =imaginal>
       +manual>
         cmd     press-key
         key     =val
       =goal>
         state   =state
         step    ready)
    '''
)

m.add_production(
    '''
    (p read-arg2
       =goal>
         isa        task
         step       retrieving-operator
       =imaginal>
       =retrieval>
         isa        operator
         action     read
         arg2       fill
         post       =state
       =visual-location>
       ?visual>
         state      free
       ?imaginal>
         state      free
      ==>
       +visual>
         cmd        move-attention
         screen-pos =visual-location
       *imaginal>
         arg2       fill
       =goal>
         step       attending
         state      =state)
    '''
)

m.add_production(
    '''
    (p encode-arg2
       =goal>
         isa    task
         step   attending
       =imaginal>
         isa    args
         arg2   fill
       =visual>
         isa    text
         value  =val
       ?imaginal>
         state  free
      ==>
       *imaginal>
         arg2   =val
       =goal>
         step   ready)
    '''
)

m.add_production(
    '''
    (p complete-task
       =goal>
         isa     task
         step    retrieving-operator
       =retrieval>
         isa     operator
         action  complete-task
         post    =state
       ==>
       +goal>
         isa     task
         state   =state
         step    ready)
    '''
)
m.spp('complete-task', **{'reward': 20})

m.goal_focus(isa='task', state='start', step='ready')


if __name__ == "__main__":
    print(m.run(10.0), end="")
