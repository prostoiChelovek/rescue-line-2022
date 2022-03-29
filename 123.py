import numpy as np
import cv2.cv2 as cv


img5=cv.imread (r"C:\Users\User\Downloads\cb1\flg.jpg")

img4=cv.imread (r"C:\Users\User\Downloads\cb1\flg1.jpg")

img3=cv.imread (r"C:\Users\User\Downloads\cb1\flg2.jpg")

img2=cv.imread (r"C:\Users\User\Downloads\cb1\flgg.jpg")

img=cv.imread (r"C:\Users\User\Downloads\cb1\flgg1.jpg")
img1=cv.imread (r"C:\Users\User\Downloads\cb1\flgg2.jpg")
print(np.mean(img))
print(np.mean(img1))
print(np.mean(img2))
print(np.mean(img3))
print(np.mean(img4))
print(np.mean(img5))