"""Python port of ACT-R:tutorial;unit8;simple-ppm-model.lisp."""

import rust_actr as actr

m = actr.Model('test-ppm')

m.sgp('v', True)
m.sgp('trace_detail', 'high')
m.sgp('esc', True)
m.sgp('ppm', None)
m.sgp('egs', 0)
m.sgp('cst', True)

m.add_chunk('small', 'chunk')
m.add_chunk('medium', 'chunk')
m.add_chunk('large', 'chunk')

m.chunk_type('test', 'value')

m.set_similarities([('small', 'medium', -0.5), ('medium', 'large', -0.5)])

m.goal_focus(isa='test', value='small')

m.add_production(
    '''
    (p small
       =goal>
         isa test
         value small
         value =val
       ==>
       !output! =val
       -goal>)
    '''
)

m.add_production(
    '''
    (p medium
       =goal>
         isa test
         value medium
         value =val
       ==>
       !output! =val
       -goal>)
    '''
)

m.add_production(
    '''
    (p large
       =goal>
         isa test
         value large
         value =val
       ==>
       !output! =val
       -goal>)
    '''
)


if __name__ == "__main__":
    print(m.run(10.0), end="")
