# Keyboard

This repo contains my attempts at optimizing and creating my own keyboard.

It is aware of the concept of layers and has metrics that work with layers.

## Metrics

Metrics are split into three categories: letters, bigrams, and trigrams,
by what kind of data is required to evaluate them. I haven't yet implemented
a single trigram metric, since the multikey nature of layered layouts require
rethinking of the alternation, rolling, and redirection metrics otherwise
used in normal keyboard layouts.

- Letter
  - `base`: a weighted measure of how much effort it takes to press this key
    assuming the hand is in the resting position (fingers in base positions)
  - `stretch`: a weighted measure of how much further apart the fingers are
    pressing this key than in the resting position
- Bigram
  - `sfb`: the amount of same finger bigrams
  - `movement`: a weighted measure of how much movement is required to move
    from one character to the next
  - `staccato`: a measure of "staccato tax", a measure of the amount of stutter
    induced by having to release held keys

## Evaluation

The evaluation currently used is made of a few steps, and requires a reference
layout and the starting layout. The reference and the starter are both evaluated 
beforehand.

There are three kinds of evaluation: 
- raw evaluation
- weighted evaluation
- normalized evaluation

Each one builds on the output of the last one.

The raw evaluation is simply the raw metric outputs computed by the algorithms
corresponding to each metric.

To calculate the weighted evaluation of a layout, the reference layout's raw
evaluation is required. The layout's raw eval is calculated, divided by the
reference layout's raw eval, then multiplied by 100 to ultimate get percentages.
Then, the relevant metrics are extracted, squared, weighted, and summed.

To calculate the normalized (final) evaluation of a layout, the starter layout's
weighted evaluation is required. The layout's weighted eval is calculated, then
multiplied by a scale factor such that the starter layout has value 1,000,000.

Ultimately, the goal of all this is to achieve a few things:
- Even improvement in all relevant metrics: the squared sum used for weighted
  eval should penalize uneven improvement.
- Consistent meaning of temperature: By keeping the normalized eval normalized,
  the temperature should behave the same no matter what the evaluation of the
  current and starting layouts are.
- Human readable: The few scales in there are to make each scale more useful
  for humans, such as the 1000000 factor removing decimal places.

## Optimization

The optimization is fundamentally simulated annealing.

The mutation has five operations:
- swap two random hold behaviors on base layer
- swap two random tap behaviors on random layer
- swap two tap behaviors vertically between two random layers
- assign new tap behavior in random spot
- assign new hold behavior in random spot (normally disabled)

In each step, the current state is mutated, then its validity is checked, i.e.
whether every character I need can be reached. If it cannot, then the state
is reverted and a new mutation is done, until it's valid.

This validity step is also where certain constraints may be added, such as:
- all letter keys must be on the base layer
- all number keys must be on the same layer
- [TODO] parens/brackets/braces must be paired
- [TODO?] numbers must be in the same row

Once a valid mutation is found, its new evaluation is calculated, and the result
is used in the probability of acceptance function to find the chance of acceptance.

If it is accepted, then the state is changed, and a cleanup step is done, where:
- Useless keys have a chance of being removed
- Useless layers may be removed (detrimental for unknown reasons)

And that concludes an optimization step.

