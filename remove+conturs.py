import cv2 as cv
import numpy as np
from PIL import Image, ImageEnhance
im = Image.open(r"C:\Users\User\Downloads\cb1\ballli.jpg")
# Создание объекта класса Contrast
im3=ImageEnhance.Contrast(im)
contrImg=im3.enhance(3.5)
contrImg.save(r"C:\Users\User\Downloads\cb1\147.jpg")
img = cv.imread(r"C:\Users\User\Downloads\cb1\147.jpg",0)
kernel = np.ones((25, 25), 'uint8')# создаем ядром 25/25
erode_img = cv.erode(img, kernel)# размываем
hist,bins = np.histogram(erode_img.ravel(),4,[0,256])#создаем гистограмму с размытым img
maxpick=max(hist)#самый длинный столбец
x=np.histogram(erode_img, maxpick)#усл со столбцом
y=x[1][np.argmax(x[0])]#возвращает максимальное значение по x и y
#делаем чб img значение 0 не может быть присвоено по какой-то причине, иначе всё становится белым
erode_img[np.where(erode_img>y)]=1
erode_img[np.where(erode_img<=y)]=255
kernel = np.ones((25, 25), 'uint8')
erode_img1 = cv.erode(erode_img, kernel)
ret, thresh = cv.threshold(erode_img1, 250, 255, 0)# порог цвета определяемого объекта
Moments = cv.moments(thresh)
#вычисляем центр белой фигуры
x = int(Moments["m10"] / Moments["m00"])
y = int(Moments["m01"] / Moments["m00"])
ret1, thresh1 = cv.threshold(erode_img1, 0, 255, 0)
#вычисляем центр всего img
Moments1 = cv.moments(thresh1)
x1 = int(Moments1["m10"] / Moments1["m00"])
y1 = int(Moments1["m10"] / Moments1["m00"])
#рисуем эти цетры
cv.circle(erode_img1, (x, y), 8, (100, 100, 100), -1)
cv.circle(erode_img1, (x1, y1), 10, (100, 100, 100), -1)
cv.imshow('Eroded Image1', erode_img1)
cv.waitKey(0)
cv.destroyAllWindows()
#вычисляем растояние от центра всей картинки до центра белой фигуры
distance=x1-x
print(distance, "растояние от центра всей картинки до центра белой фигуры")
#определяем круг или прямоугольник, если круг, то разность x и y  будет примерно равна 1
w=np.where(img==0)
x=max(w[1])-min(w[1]+1)
y=max(w[0])-min(w[0]+1)
yx=y/x
if yx> 1.5 or yx<0.5:
    print(yx,"прямоугольник")
else:
    print(yx,"круг")
#определяем растояние крайних пикселей, чем оно больше, тем ближе машинка к объекту
w=np.where(img==0)
x=max(w[1])-min(w[1]+1)
print(x, "растояние крайних пикселей, чем оно больше, тем ближе машинка к объекту")