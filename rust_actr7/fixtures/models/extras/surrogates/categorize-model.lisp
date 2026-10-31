(clear-all)
(define-model categorize-surrogate
  (sgp :v t :act t)
  (sgp :esc t :lf .01 :blc 10 :rt -100 :do-not-harvest imaginal)
  (sgp :sim-hook "size-similarities" :cache-sim-hook-results t)
  (sgp :mp 1 :ppm 1 :egs .25 :ans .25)
  (chunk-type goal state name value)
  (chunk-type example category eh es nl mh)
  (add-dm (small isa chunk) (medium isa chunk) (large isa chunk))
(add-dm
 (example0 isa example category 1 eh small es large nl medium mh small)
 (example1 isa example category 1 eh medium es small nl large mh medium)
 (example2 isa example category 1 eh small es medium nl medium mh medium)
 (example3 isa example category 1 eh small es medium nl large mh medium)
 (example4 isa example category 1 eh small es medium nl large mh large)
 (example5 isa example category 2 eh large es small nl small mh small)
 (example6 isa example category 2 eh medium es medium nl small mh large)
 (example7 isa example category 2 eh large es small nl large mh small)
 (example8 isa example category 2 eh large es small nl large mh large)
 (example9 isa example category 2 eh large es small nl small mh large))
  (define-chunks unknown categorize add-attribute retrieving eh es nl mh)
  (set-similarities (small medium -.4) (medium large -.4) (small large -.8))
  (set-buffer-chunk 'imaginal '(category unknown))
  (declare-buffer-usage goal goal :all)

  (p encode-small
    =goal> isa goal state add-attribute name =name value small value =value
    =imaginal>
    ==>
    =imaginal> =name small
    -goal>)

  (p encode-medium
    =goal> isa goal state add-attribute name =name value medium value =value
    =imaginal>
    ==>
    =imaginal> =name medium
    -goal>)

  (p encode-large
    =goal> isa goal state add-attribute name =name value large value =value
    =imaginal>
    ==>
    =imaginal> =name large
    -goal>)

  (p retrieve-example
    =goal> isa goal state categorize
    =imaginal>
    ?retrieval> state free
    ==>
    +retrieval> =imaginal
    =goal> state retrieving)

  (p set-category
    =goal> isa goal state retrieving
    =imaginal>
    =retrieval> category =cat
    ?imaginal> state free
    ==>
    =imaginal> category =cat
    -goal>)
)
