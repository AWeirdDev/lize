import lize
from pickle import dumps

print(len(lize.serialize([100000000000, 10, 10, 10, 10, 50])))
print(len(dumps([100000000000, 10, 10, 10, 10, 50])))
