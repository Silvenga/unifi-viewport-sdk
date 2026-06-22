# Camera Layout

These are the layouts as shown in the Protect console. Each layout is hardcoded. Layouts 1-16 appear to be supported.

> The actual stream locations aren't known yet (requires some amount of testing).
> The numbers are purely here to show which sections are different cameras.

**1**

```
1
```

**2**

```
1 | 2
```

**3**

```
1 | 2
  | 3
```

**4**

```
1 | 2
3 | 4
```

**5**

```
1     | 2 | 3
      | 4 | 5
```

**6**

```
1     | 2
      | 3
4 | 5 | 6
```

**7**

```
1     | 2
      |
3     | 4 | 5
      | 6 | 7
```

**8**

```
1   | 2   | 3
    |     | 4
5   | 6   | 7
    |     | 8
```

**9**

```
1  | 2  | 3
4  | 5  | 6
7  | 8  | 9
```

**10**

```
1      | 2  | 3
       | 4  | 5
6      | 7  | 8
       | 9  | 10
```

**11**

```
1       | 2       | 3
        |         | 4
5       | 6  | 7  | 8
        | 9  | 10 | 11
```

**12**

```
1           | 2
            |
            |
3           | 4  | 5  | 6
            | 7  | 8  | 9
            | 10 | 11 | 12
```

**13**

```
1       | 2  | 3
        | 4  | 5
6  | 7  | 8  | 9
10 | 11 | 12 | 13
```

**14**

```
1    | 2  | 3  | 4
     | 5  | 6  | 7
8    | 9  | 10 | 11
     | 12 | 13 | 14
```

**15**

```
1            | 2
             |
3     | 4    | 7  | 8  | 9
5     | 6    | 10 | 11 | 12
             | 13 | 14 | 15
```

> This one is a bit weird, 1 and 2 are the same size (half of the view). The cell for [3/4/5/6] is the same size
> as [10/11/12/13/14/15]. So not perfect divisions.

**16**

```
1  | 2  | 3  | 4
5  | 6  | 7  | 8
9  | 10 | 11 | 12
13 | 14 | 15 | 16
```
