#define BITS_IN_INT (sizeof(int) * 8)

#define CHECK_BIT(x, index) ((x) & (1 << (index)))

#define APPLY_RULE(rule, leftValue, value, rightValue) (CHECK_BIT(rule, leftValue << 2 | value << 1 | rightValue))

#define ARRAY_INDEX(index) ((index) / BITS_IN_INT)
#define INT_INDEX(index) ((index) % BITS_IN_INT)

