"""Python port of ACT-R:tutorial;unit8;paired-dynamic-model.lisp."""

import rust_actr as actr

m = actr.Model('paired-learning-dynamic')

m.sgp(':esc', True)
m.sgp(':v', True)
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
m.sgp(':do-not-harvest', 'imaginal')

m.chunk_type('operator', 'pre', 'action', 'arg1', 'arg2', 'post', 'label', 'required', 'slot', 'success', 'failure')
m.chunk_type('task', 'state', 'step', 'context')

m.define_chunks('stimulus-read')
m.define_chunks('read')
m.define_chunks('start')
m.define_chunks('recalled')
m.define_chunks('number')
m.define_chunks('retrieve')
m.define_chunks('wait')
m.define_chunks('respond')
m.define_chunks('type')
m.define_chunks('new-trial')
m.define_chunks('complete-task')
m.define_chunks('ready')
m.define_chunks('retrieving-operator')
m.define_chunks('process')
m.define_chunks('retrieving-result')
m.add_chunk('op1', 'operator', pre='start', action='read', label='word', post='stimulus-read')
m.add_chunk('op2', 'operator', pre='stimulus-read', action='retrieve', required='word', label='number', post='recalled')
m.add_chunk('op3', 'operator', pre='recalled', slot='number', success='respond', failure='wait')
m.add_chunk('op4', 'operator', pre='respond', action='type', required='number', post='wait')
m.add_chunk('op5', 'operator', pre='wait', action='read', label='number', post='new-trial')
m.add_chunk('op6', 'operator', pre='new-trial', action='complete-task', post='start')

m.set_all_base_levels(1000)

m.goal_focus(isa='task', state='start', step='ready')

m.add_production(
    '''
    (p retrieve-operator
       =goal>
         isa     task
         state   =state
         step    ready
      ==>
       +retrieval>
         isa     operator
         pre     =state
       =goal>
         step    retrieving-operator
         context nil)
    '''
)

m.add_production(
    '''
    (p read-and-create
       =goal>
         isa       task
         step      retrieving-operator
       =retrieval>
         isa       operator
         action    read
         label     =destination
         post      =state
       =visual-location>
       ?visual>
         state     free
       ?imaginal>
         buffer    empty
         state     free
      ==>
       +imaginal>
       +visual>
         isa        move-attention
         screen-pos =visual-location
       =goal>
        context    =destination
        step       process
        state      =state)
    '''
)

m.add_production(
    '''
    (p read
       =goal>
         isa        task
         step       retrieving-operator
       =retrieval>
         isa        operator
         action     read
         label      =destination
         post       =state
       =visual-location>
       ?visual>
         state      free
       ?imaginal>
         buffer     full
      ==>
       +visual>
         isa        move-attention
         screen-pos =visual-location
       =goal>
         context    =destination
         step       process
         state      =state)
    '''
)

m.add_production(
    '''
    (p encode
       =goal>
         isa         task
         step        process
         context     =which-slot
       =visual>
         isa         text
         value       =val
       =imaginal>
       ?imaginal>
         state       free
      ==>
       *imaginal>
         =which-slot =val
       =goal>
         step        ready)
    '''
)

m.add_production(
    '''
    (p retrieve-associate
       =goal>
         isa      task
         step     retrieving-operator
       =imaginal>
         =target  =stimulus
       =retrieval>
         isa      operator
         action   retrieve
         required =target
         label    =other
         post     =state
      ==>
       +retrieval>
         =target  =stimulus
       =goal>
         step     retrieving-result
         context  =other
         state    =state)
    '''
)

m.add_production(
    '''
    (p retrieval-unsuccessful
       =goal>
         isa      task
         step     retrieving-result
         context  =dest
       =imaginal>
       ?retrieval>
         buffer   failure
       ?imaginal>
         state    free
      ==>
       *imaginal>
         =dest    nil
       =goal>
         step     ready)
    '''
)

m.add_production(
    '''
    (p retrieval-successful
       =goal>
         isa      task
         step     retrieving-result
         context  =dest
       =imaginal>
       =retrieval>
         =dest    =val
       ?imaginal>
         state    free
      ==>
       *imaginal>
         =dest    =val
       =goal>
         step     ready)
    '''
)

m.add_production(
    '''
    (p unsuccessful-test
       =goal>
         isa     task
         step    retrieving-operator
       =imaginal>
         =dest   nil
       =retrieval>
         isa     operator
         slot    =dest
         failure =state
       ?imaginal>
         state   free
      ==>
       =goal>
         state   =state
         step    ready)
    '''
)

m.add_production(
    '''
    (p successful-test
       =goal>
         isa     task
         step    retrieving-operator
       =imaginal>
         =dest   =val
       =retrieval>
         isa     operator
         slot    =dest
         success =state
       ?imaginal>
         state   free
      ==>
       =goal>
         state   =state
         step    ready)
    '''
)

m.add_production(
    '''
    (p type
       =goal>
         isa      task
         step     retrieving-operator
       =imaginal>
         =slot    =val
       =retrieval>
         isa      operator
         action   type
         required =slot
         post     =state
       ?manual>
         state    free
      ==>
       +manual>
         cmd      press-key
         key      =val
       =goal>
         state    =state
         step     ready)
    '''
)

m.add_production(
    '''
    (p complete-task
       =goal>
         isa    task
         step   retrieving-operator
       =retrieval>
         isa    operator
         action complete-task
         post   =state
       ?imaginal>
         state  free
      ==>
       -imaginal>
       +goal>
         isa    task
         state  =state
         step   ready)
    '''
)
m.spp('complete-task', **{'reward': 20})


if __name__ == "__main__":
    print(m.run(10.0), end="")
