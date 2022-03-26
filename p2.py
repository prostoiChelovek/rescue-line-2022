
# выделяем зеленые квадратики
import numpy as np
import cv2 as cv
img = cv.imread(r"C:\Users\User\Downloads\cb1\a.pmg.png", 1)
# You can access a pixel value by its row and column coordinates. For BGR image, it returns an array of Blue, Green, Red values. For grayscale image, just corresponding intensity is returned.
arr = np.asarray(img, dtype='uint8')  # преобразует картинку в масив
red = img[:, :, 2]
green = img[:, :, 1]
blue0 = img[:, :, 0]

img[np.where(red >= 230)] = 0


red = img[:, :, 2] = 0
green = img[:, :, 1]
blue0 = img[:, :, 0] = 0


aq = img[:, :]
cv.imshow("window_name21", img)
cv.waitKey(0)
cv.destroyAllWindows()
print(aq)
cv.imwrite(r"C:\Users\User\Downloads\cb1\asd.jpg", img)
# нельзя в jpg
cv.split(img)
b, g, r = cv.split(img)

print(g > 0)
a0 = g[4:8, 4:8]
b0 = g[0:4, 4:8]
c0 = g[4:8, 0:4]
d0 = g[0:4, 0:4]


np.matrix(d0)
np.matrix(a0)
np.matrix(b0)
np.matrix(c0)
print(np.average(d0))
print(np.average(a0))
print(np.average(b0))
print(np.average(c0))
if np.average(d0) > 90 and np.average(d0) < 100:
    print("праваВниз")
if np.average(a0) > 90 and np.average(a0) < 100:
    print("правВерх")

if np.average(b0) > 90 and np.average(b0) < 100:
    print("левВерх")

if np.average(c0) > 90 and np.average(c0) < 100:
    print("левВниз")

if np.average(a0) > 100:
    print("поворот")

if np.average(b0) > 100:
    print("поворот")

if np.average(c0) > 100:
    print("поворот")

if np.average(d0) > 100:
    print("поворот")
