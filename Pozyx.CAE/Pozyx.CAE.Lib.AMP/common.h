#define CHECK_BIT(x, index) ((x) & (1 << (index)))

#define APPLY_RULE(rule, leftValue, value, rightValue) (CHECK_BIT(rule,(leftValue ? 4 : 0) + (value ? 2 : 0) + (rightValue ? 1 : 0)))