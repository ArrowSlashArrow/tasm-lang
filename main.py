def check_palindrome(n):
    if n < 100000:  # 5 digit
        # check 1st and 5th digit
        first = n % 10
        fifth = int(n / 10000)
        if first != fifth:
            return False

        second = int(n / 10) % 10
        fourth = int(n / 1000) % 10
        return second == fourth
    else:
        # check 1st and 6th digit
        first = n % 10
        sixth = int(n / 100000)
        if first != sixth:
            return False

        second = int(n / 10) % 10
        fifth = int(n / 10000) % 10
        if second != fifth:
            return False

        third = int(n / 100) % 10
        fourth = int(n / 1000) % 10
        return third == fourth

max_pdrome = 0

x = 100
while x < 999:
    y = 100
    while y < 999:
        i = x * y
        if n := check_palindrome(i) and i > max_pdrome:
            max_pdrome = i
        y += 1
    x += 1

print(max_pdrome)