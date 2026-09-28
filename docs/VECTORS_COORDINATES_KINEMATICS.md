# Vectors, coordinates, and kinematics

Goblin++ 0.1.0-alpha.15 adds a compact, dimension-checked foundation for
ordinary Euclidean vector calculations and introductory kinematics. The
interpreter and native compiler implement the same functions.

## Distance conversions

The conversion names state both the source and destination:

```goblin
earth_sun = au2m(1)
same_distance = m2au(earth_sun)
nearest_parsec = pc2m(1)
one_light_year = ly2m(1)
```

`au2m`, `pc2m`, and `ly2m` require a dimensionless numeric value and return a
length in SI metres. `m2au`, `m2pc`, and `m2ly` require a length and return a
dimensionless number. Goblin++ uses the exact IAU astronomical-unit definition
of 149,597,870,700 metres, the exact light-year implied by the speed of light
and a Julian year, and `648000 / pi` astronomical units per parsec.

These explicit conversions avoid treating `AU`, `pc`, or `ly` as silent aliases
inside arbitrary unit expressions.

## Vectors

Vectors are homogeneous arrays: all components must have compatible kinds and
dimensions.

```goblin
size = magnitude([3 m, 4 m])
projection = dot([1 m, 2 m, 3 m], [4 kg, 5 kg, 6 kg])
normal = cross([1 m, 0 m, 0 m], [0 kg, 1 kg, 0 kg])
```

- `magnitude(vector)` accepts any non-empty numeric vector whose components
  share a dimension.
- `dot(left, right)` accepts non-empty vectors of equal length.
- `cross(left, right)` accepts two three-component vectors.

The result dimensions are derived, not guessed. For example, length dotted
with mass has dimension `kg*m`.

## Coordinate conventions

The suffix states the angle convention: `d` is degrees and `r` is radians.

```goblin
xy = polar2cartesiand(2 m, 90)
xyz = spherical2cartesiand(2 m, 90, 0)

r = cartesian_radius(xyz)
azimuth = cartesian_azimuthd(xyz)
inclination = cartesian_inclinationd(xyz)
```

Goblin++ uses these conventions:

- Cartesian axes are `[x, y]` or `[x, y, z]`.
- Polar input is `(radius, azimuth)`, measured from positive x toward positive
  y.
- Spherical input is `(radius, inclination, azimuth)`.
- Inclination is measured down from positive z: 0 degrees is +z and 90 degrees
  is the x-y plane.
- Azimuth is measured in the x-y plane from positive x toward positive y.

`polar2cartesiand`/`polar2cartesianr` return `[x, y]`.
`spherical2cartesiand`/`spherical2cartesianr` return `[x, y, z]`.
The inverse helpers return one homogeneous value at a time because Goblin++
does not place a length and an angle into a single mixed-dimension array.
Azimuth at `x = y = 0`, inclination at the origin, negative radii, and
out-of-range spherical inclinations are refused explicitly.

## Linear and rotational motion

```goblin
speed = velocity(10 m, 2 s)
vector_speed = velocity([10 m, 4 m, 0 m], 2 s)

ordinary_sum = velocity_add_galilean(speed, velocity(6 m, 2 s))
relativistic_sum = velocity_add_relativistic_collinear(c / 2, c / 2)

omega = angular_velocityd(360, 2 s)
tangent_speed = tangential_velocity(2 m, omega)
inward_acceleration = centripetal_acceleration(2 m, omega)
```

`velocity` calculates displacement divided by a strictly positive elapsed
time. `velocity_add_galilean` accepts two scalar velocities or two
same-length velocity vectors. `velocity_add_relativistic_collinear` implements
the one-dimensional special-relativistic formula `(u + v) / (1 + uv/c^2)`;
its name deliberately does not pretend to handle arbitrary three-dimensional
frame transformations.

`angular_velocityd` converts degrees per elapsed time to radians per second;
`angular_velocityr` accepts radians. Radians are dimensionless in the current
SI dimension model, so angular velocity renders as `s^-1`. Use
`tangential_velocity(radius, omega)` for `r * omega` and
`centripetal_acceleration(radius, omega)` for `r * omega^2`.

Angular momentum uses the right-handed cross product:

```goblin
L1 = angular_momentum(position, momentum)
L2 = angular_momentum_from_velocity(position, mass, velocity_vector)
```

Both functions require three-component vectors and return `r cross p` with
dimension `kg*m^2/s`.

## Scope and safety boundary

The existing Goblin++ dimensional engine checks every quantity passed to these
functions. This module does not duplicate or bypass it.

These helpers model Euclidean Cartesian geometry, elementary kinematics,
rigid circular motion, and one-dimensional special-relativistic velocity
addition. They do **not** implement reference frames, tensors, orbital
propagation, general relativity, curved spacetime, or uncertainty propagation.
Those need their own explicit models and independently validated acceptance
data; a vague `curved_motion()` function would be a very well-dressed goblin.

